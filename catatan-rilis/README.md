# Catatan rilis

Satu berkas per versi: `<versi>.md` (misalnya `2.5.0.md`). Workflow *Rilis* memakainya sebagai isi
draf rilis ketika tag `v<versi>` di-push, dan mengganti baris yang hanya berisi `@@CHECKSUM@@`
dengan SHA-256 installer Windows dan .dmg macOS.

Susunan yang dipakai selama ini: satu kalimat ringkasan, *Yang baru* / *Yang diperbaiki*, cara
memasang atau memperbarui, lalu:

````markdown
## Checksum

```
@@CHECKSUM@@
```
````

Catatan rilis ditulis dalam ejaan sekarang (bukan ejaan README).
