# Pacman Repository Notes

Build an Arch package, sign it, and add it to a local repository:

```sh
makepkg -Csf
gpg --detach-sign --use-agent ardali-gaming-0.1.0-1-x86_64.pkg.tar.zst
repo-add --sign ardali.db.tar.gz ardali-gaming-0.1.0-1-x86_64.pkg.tar.zst
```

Example client config:

```ini
[ardali]
SigLevel = Required DatabaseOptional
Server = https://repo.example.invalid/ardali/$arch
```

Replace the URL, maintainer, source archive, checksums, and signing key before publishing.
