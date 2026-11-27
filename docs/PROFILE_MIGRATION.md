# Copy a development profile to Praetorium

Close both applications. Keep the original Elite Nexus and EliteNexus-Gen2 installations and their profiles for rollback. Praetorium uses its own profile. The app never imports a prior profile on startup.

Run the copy tool using Python 3.10 or newer. Explicitly select the closed EliteNexus-Gen2 profile and a new destination on a local filesystem that supports hard links. The destination's parent must exist and the destination itself must be absent.

```powershell
python tools/migrate-profile.py --source 'C:\path\to\gen2-profile' --destination 'C:\path\to\praetorium-profile'
```

The tool takes the named UI and speech preferences and copies a compatible galaxy database if one is present. It copies relevant SQLite sidecars into a temporary staging area and checks the copy before publishing. It excludes credentials, upload queues, consent, AI setup, protected paths, logs and journal preferences. It leaves the original files in place. Review the new profile before launching. For an isolated Windows run, use the separate binary and select the journal folder explicitly.

```powershell
$env:PRAETORIUM_HOME = 'C:\path\to\praetorium-profile'
& 'C:\path\to\Praetorium.exe' --isolated --mute --journal-dir 'C:\path\to\journals'
Remove-Item Env:PRAETORIUM_HOME
```

To roll back, close Praetorium and launch the earlier application with its own executable and profile. If a migration is interrupted, keep the original pair and review the new destination before retrying with a fresh destination.
