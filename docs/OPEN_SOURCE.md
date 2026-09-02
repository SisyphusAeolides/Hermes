# Hermes source and firmware policy

Hermes' implementation is source-available under the MIT license in this
repository. That includes the Rust crates, C kernel-module surface, formal
models, build scripts, generated interface tables, and compatibility headers.
There are no proprietary driver sources or prebuilt driver objects in the
repository.

Hermes is a clean-room implementation informed by public Linux, Nouveau, Nova,
and vendor documentation. Public interfaces and protocol facts may be
reimplemented; proprietary source code is never copied into this tree.

The public reference points are the [Nouveau project](https://nouveau.freedesktop.org/)
and the Linux [Nova DRM documentation](https://docs.kernel.org/gpu/nova/index.html).
Hermes does not copy either project’s code; the references document the
interfaces and architectural decisions that the clean-room implementation
recreates.

GPU firmware is a separate artifact. OpenRM/GSP, AMD PSP/SMU, and Intel GuC/HuC
images are supplied by the hardware vendor or a distribution firmware package
under their own terms. Hermes stores only the version, length, digest, and
structural checks needed to authenticate an operator-staged image. Firmware is
not silently downloaded, embedded, or redistributed by Hermes.

`scripts/audit-open-source.sh` is the release check for this boundary. It
rejects tracked binary driver artifacts, missing package license declarations,
and Git LFS placeholders. A green audit means the Hermes implementation is
open source; it does not turn separately licensed firmware into MIT software.
