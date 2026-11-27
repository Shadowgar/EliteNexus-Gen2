//! The "Ask" assistant: answers questions on a worker thread with `nexus-ai`, using tools over the
//! app's own data. The UI only polls for progress.

use crate::ai_tools::{CommanderScope, NexusTools, TOOLS, tool};
use crate::settings::AiSettings;
use crossbeam_channel::{Receiver, TryRecvError};
use nexus_ai::{AiError, Answer, Backend, Destination, ProviderConfig, ProviderHttp, ProviderKind, Remote, Reply, Step, ToolHost, ToolSpec, Turn, Usage};
use nexus_model::Model;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// One question and its answer.
pub struct Exchange {
    pub question: String,
    /// None while the assistant is working, and after it was stopped.
    pub answer: Option<Result<String, String>>,
    pub tools: Vec<String>,
    pub usage: Usage,
    /// The commander stopped it before it answered.
    pub stopped: bool,
}

#[derive(Clone, PartialEq, Eq)]
struct ConversationScope {
    provider: ProviderKind,
    destination: Option<Destination>,
    credential_generation: u64,
    model: String,
    commander: CommanderScope,
    source_generation: u64,
}

enum Event {
    Step(Step),
    Done(Result<Answer, AiError>, Option<String>, Vec<Turn>),
}

struct Pending {
    generation: u64,
    stop: Arc<AtomicBool>,
    events: Receiver<Event>,
}

#[derive(Default)]
pub struct AssistantService {
    pub exchanges: Vec<Exchange>,
    pub status: Option<String>,
    pub model: Option<String>,
    scope: Option<ConversationScope>,
    source_generation: u64,
    generation: u64,
    conv: Vec<Turn>,
    pending: Option<Pending>,
    http: ProviderHttp,
}

impl AssistantService {
    /// A sample conversation for screenshot runs (`--demo`): long lines, a list and a pending answer.
    pub fn demo() -> Self {
        let mut s = AssistantService { model: Some("gemini-3.5-flash-lite".into()), ..Default::default() };
        s.exchanges.push(Exchange {
            question: "Are you aware of the first footfall status of planets on the route we're planning for exobiology? I need planets where I can find expensive biology that nobody has walked on yet.".into(),
            answer: Some(Ok("Yes. The exobiology planner (powered by Spansh) targets bodies that typically carry high-value biology such as **Stratum Tectonicas**, worth about 19 M Cr base or about 95 M Cr with the first-logged bonus.\n\nTo get the ×5 bonus:\n- Pick systems a little off the beaten path, where no commander has stamped their name.\n- When you scan a body with the FSS or DSS, check its footfall status before you land; bodies nobody has walked on in unpopulated systems qualify.\n  - Populated systems never pay the bonus, even when the body shows no footfall.\n\nOpen the Navigation page to plan the trip. Happy hunting!".into())),
            tools: vec!["system_bodies".into(), "knowledge".into()],
            usage: Usage::default(),
            stopped: false,
        });
        s.exchanges.push(Exchange { question: "Which of those bodies are still unfootfalled?".into(), answer: None, tools: Vec::new(), usage: Usage::default(), stopped: false });
        s.status = Some("Checking the planets…".into());
        // Busy, as while a question runs (a channel nobody sends on), so the screenshot shows Stop.
        s.pending = Some(Pending { generation: s.generation, stop: Arc::default(), events: crossbeam_channel::never() });
        s
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Refresh the app-owned authorization before sending or applying worker results.
    pub fn sync_scope(&mut self, ai: &AiSettings, model: &Model) -> bool {
        let scope = ConversationScope {
            provider: ai.provider,
            destination: ai.destination().ok(),
            credential_generation: ai.credential_generation(),
            model: ai.model.trim().to_string(),
            commander: CommanderScope::capture(model),
            source_generation: self.source_generation,
        };
        let changed = self.scope.as_ref().is_some_and(|old| *old != scope);
        if changed {
            self.clear();
        }
        self.scope = Some(scope);
        changed
    }

    pub fn replace_source(&mut self) {
        self.source_generation = self.source_generation.wrapping_add(1);
        self.clear();
        self.scope = None;
    }

    pub fn stop(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.stop.store(true, Ordering::Release);
            self.generation = self.generation.wrapping_add(1);
            self.status = None;
            if let Some(last) = self.exchanges.last_mut() {
                last.stopped = true;
            }
        }
    }

    pub fn clear(&mut self) {
        self.stop();
        self.generation = self.generation.wrapping_add(1);
        self.exchanges.clear();
        self.conv.clear();
        self.model = None;
    }

    pub fn ask(&mut self, question: String, cfg: ProviderConfig, mut tools: NexusTools, context: String, ctx: &egui::Context) {
        if !model_service_allowed(crate::launch::isolated(), &cfg) {
            self.status = None;
            self.exchanges.push(Exchange {
                question,
                answer: Some(Err(if crate::launch::isolated() { "Model connections are unavailable in isolated mode." } else { "The model address is invalid. Use HTTPS or a loopback HTTP address." }.into())),
                tools: Vec::new(),
                usage: Usage::default(),
                stopped: false,
            });
            return;
        }
        let question = question.trim().to_string();
        let Some(scope) = self.scope.clone() else { return };
        if self.busy() || question.is_empty() || !cfg.is_ready() || scope.destination != cfg.destination().ok() {
            return;
        }
        self.exchanges.push(Exchange { question: question.clone(), answer: None, tools: Vec::new(), usage: Usage::default(), stopped: false });
        self.status = Some("Thinking...".into());
        let (tx, events) = crossbeam_channel::unbounded();
        let stop = Arc::new(AtomicBool::new(false));
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(Pending { generation: self.generation, stop: stop.clone(), events });
        tools.commander_scope = Some(scope.commander.clone());
        let (http, ctx, cached, mut turns) = (self.http.clone(), ctx.clone(), self.model.clone(), self.conv.clone());
        let spawned = std::thread::Builder::new().name("assistant".into()).spawn(move || {
            let send = |e: Event| {
                let _ = tx.send(e);
                ctx.request_repaint();
            };
            let host = Guarded { tools: &tools, stop: &stop, scope: &scope };
            if !host.current() {
                return;
            }
            let rt = &tools.rt;
            let model = if let Some(model) = cached {
                Ok(model)
            } else if !cfg.model.is_empty() {
                Ok(cfg.model.clone())
            } else {
                match nexus_ai::list_models(&http, rt, &cfg, &stop) {
                    Ok(list) => nexus_ai::resolve_model(&cfg, &list).ok_or(AiError::NoModel),
                    Err(e) if !e.is_auth() && e != AiError::Stopped => nexus_ai::resolve_model(&cfg, &[]).ok_or(e),
                    Err(e) => Err(e),
                }
            };
            let model = match model {
                Ok(model) => model,
                Err(error) => {
                    send(Event::Done(Err(error), None, Vec::new()));
                    return;
                }
            };
            let remote = Remote { http: &http, rt, cfg: &cfg, model: model.clone(), stop: Some(&stop) };
            let backend = ScopedBackend { remote, host: &host };
            nexus_ai::trim_history(&mut turns, 6);
            let question = nexus_ai::with_context(&context, &question);
            let answer = nexus_ai::ask(&backend, nexus_ai::SYSTEM_PROMPT, &host, &mut turns, question, &stop, |s| send(Event::Step(s)));
            if host.current() {
                send(Event::Done(answer, Some(model), turns));
            }
        });
        if spawned.is_err() {
            self.pending = None;
            self.status = None;
            if let Some(last) = self.exchanges.last_mut() {
                last.answer = Some(Err("Could not start the assistant.".into()));
            }
        }
    }

    /// Publish only on the app thread, after checking the live scope.
    pub fn poll(&mut self) {
        loop {
            let Some(pending) = &self.pending else { return };
            if pending.generation != self.generation {
                self.stop();
                return;
            }
            match pending.events.try_recv() {
                Ok(Event::Step(Step::Thinking)) => self.status = Some("Thinking...".into()),
                Ok(Event::Step(Step::Tool(name))) => {
                    self.status = Some(crate::ai_tools::working(&name).into());
                    if let Some(last) = self.exchanges.last_mut()
                        && !last.tools.contains(&name)
                    {
                        last.tools.push(name);
                    }
                }
                Ok(Event::Done(answer, model, turns)) => {
                    self.model = if answer.is_ok() { model } else { None };
                    if let Some(last) = self.exchanges.last_mut() {
                        match answer {
                            Ok(a) => {
                                self.conv = turns;
                                last.answer = Some(Ok(a.text));
                                last.usage = a.usage;
                                last.tools = a.tools_used;
                            }
                            Err(e) => last.answer = Some(Err(friendly(&e))),
                        }
                    }
                    self.pending = None;
                    self.status = None;
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.stop();
                    return;
                }
            }
        }
    }
}

fn model_service_allowed(isolated: bool, cfg: &ProviderConfig) -> bool {
    !isolated && cfg.destination().is_ok()
}

struct Guarded<'a> {
    tools: &'a NexusTools,
    stop: &'a AtomicBool,
    scope: &'a ConversationScope,
}

impl Guarded<'_> {
    fn current(&self) -> bool {
        let model = self.tools.model.read();
        if !self.scope.commander.matches(&model) {
            self.stop.store(true, Ordering::Release);
        }
        !self.stop.load(Ordering::Acquire)
    }
}

impl ToolHost for Guarded<'_> {
    fn tools(&self) -> Vec<ToolSpec> {
        TOOLS.iter().filter(|t| t.available_to(self.tools.caller)).map(|t| t.spec()).collect()
    }

    fn call(&self, name: &str, args: &Value) -> Result<Value, String> {
        if !self.current() {
            return Err("The conversation scope changed.".into());
        }
        if !tool(name).is_some_and(|t| t.available_to(self.tools.caller)) {
            return Err("This tool is unavailable in the current scope.".into());
        }
        let result = self.tools.call(name, args);
        if self.current() { result } else { Err("The conversation scope changed.".into()) }
    }
}

struct ScopedBackend<'a> {
    remote: Remote<'a>,
    host: &'a Guarded<'a>,
}

impl Backend for ScopedBackend<'_> {
    fn complete(&self, system: &str, tools: &[ToolSpec], turns: &[Turn]) -> Result<Reply, AiError> {
        if !self.host.current() {
            return Err(AiError::Stopped);
        }
        let reply = self.remote.complete(system, tools, turns);
        if self.host.current() { reply } else { Err(AiError::Stopped) }
    }
}

fn friendly(e: &AiError) -> String {
    match e {
        e if e.is_auth() => "The model service rejected the API key. Check it with the gear button.".into(),
        AiError::Status { code: 429, .. } => "The model service is busy or the free quota is used up. Try again in a minute.".into(),
        AiError::Status { code: 404, .. } => "The configured model or service address was not found.".into(),
        AiError::NoModel => "No model is available for this service.".into(),
        AiError::Http(m) => format!("Can't reach the model service: {m}"),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(svc: &mut AssistantService) -> (crossbeam_channel::Sender<Event>, Arc<AtomicBool>) {
        let (send, events) = crossbeam_channel::unbounded();
        let stop = Arc::new(AtomicBool::new(false));
        svc.generation += 1;
        svc.pending = Some(Pending { generation: svc.generation, stop: stop.clone(), events });
        svc.exchanges.push(Exchange { question: "fixture".into(), answer: None, tools: Vec::new(), usage: Usage::default(), stopped: false });
        (send, stop)
    }

    fn done(text: &str) -> Event {
        Event::Done(Ok(Answer { text: text.into(), tools_used: Vec::new(), usage: Usage::default() }), Some("fixture-model".into()), vec![Turn::User(text.into())])
    }

    #[test]
    fn same_scope_keeps_committed_turns_and_stop_discards_queued_result() {
        let mut svc = AssistantService::default();
        let ai = AiSettings::default();
        let model = Model::default();
        svc.sync_scope(&ai, &model);
        let (send, _) = pending(&mut svc);
        send.send(done("first")).unwrap();
        svc.poll();
        assert!(!svc.sync_scope(&ai, &model));
        assert!(matches!(&svc.conv[..], [Turn::User(text)] if text == "first"));
        let (send, stopped) = pending(&mut svc);
        send.send(done("discard")).unwrap();
        svc.stop();
        svc.poll();
        assert!(stopped.load(Ordering::Acquire));
        assert!(matches!(&svc.conv[..], [Turn::User(text)] if text == "first"));
        assert!(svc.exchanges.last().unwrap().stopped);
    }

    #[test]
    fn model_gate_preserves_configured_providers_and_blocks_isolated_sessions() {
        use nexus_ai::ProviderKind::{Claude, Custom, Gemini, Local};
        for (kind, url, expected) in [
            (Gemini, "", true), (Claude, "", true),
            (Custom, "https://api.openai.com/v1", true),
            (Local, "https://example.invalid/v1", true),
            (Local, "http://127.0.0.1:7/v1", true),
            (Custom, "http://example.invalid/v1", false),
            (Custom, "https://user:password@example.invalid/v1", false),
            (Custom, "https://example.invalid/v1?key=fixture", false),
        ] {
            let cfg = ProviderConfig::new(kind, url.into(), "fixture".into());
            assert_eq!(model_service_allowed(false, &cfg), expected);
            assert!(!model_service_allowed(true, &cfg));
        }
    }

    #[test]
    fn different_provider_commander_and_source_retire_work_and_discovery() {
        let mut svc = AssistantService::default();
        let mut ai = AiSettings::default();
        let mut model = Model::default();
        svc.sync_scope(&ai, &model);
        let (send, stopped) = pending(&mut svc);
        send.send(done("old")).unwrap();
        ai.provider = ProviderKind::Claude;
        assert!(svc.sync_scope(&ai, &model));
        svc.poll();
        assert!(stopped.load(Ordering::Acquire));
        assert!(svc.model.is_none() && svc.conv.is_empty() && svc.exchanges.is_empty());
        let (send, stopped) = pending(&mut svc);
        model.active = Some("synthetic-commander".into());
        assert!(svc.sync_scope(&ai, &model));
        assert!(stopped.load(Ordering::Acquire));
        assert!(send.send(done("discard")).is_err());
        let (_, stopped) = pending(&mut svc);
        svc.replace_source();
        assert!(stopped.load(Ordering::Acquire));
        assert!(svc.scope.is_none() && svc.exchanges.is_empty());
    }

    #[test]
    fn scope_policy_hides_history_and_side_effects_from_ai_interfaces() {
        use crate::ai_tools::Caller;
        for caller in [Caller::Ask, Caller::LocalAsk, Caller::Mcp] {
            for name in ["credit_history", "on_foot", "system_factions", "plan_route", "money_routes"] {
                assert!(!tool(name).unwrap().available_to(caller), "{name}");
            }
            assert!(tool("knowledge").unwrap().available_to(caller));
            assert!(tool("journal_search").unwrap().available_to(caller));
        }
        assert!(!tool("system_bodies").unwrap().available_to(Caller::LocalAsk));
    }
}
