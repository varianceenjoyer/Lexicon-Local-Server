# Research Plan: License Audit

**ID:** R-LIC-01
**Status:** Ongoing
**Category:** Legal
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** License compatibility report

---

## Problem Statement

The project depends on multiple open-source libraries and ML models. Before distribution (especially via app stores), we must ensure:
- All dependency licenses are compatible with the project's intended license
- No copyleft licenses that require source disclosure (if proprietary distribution planned)
- Attribution requirements are understood and met
- Model licenses permit commercial/redistributable use

**Engineering Challenge:** Audit all dependencies and models for license compatibility and document attribution requirements.

---

## Research Questions

### RQ1: What licenses do core Rust dependencies use?

**Dependencies to audit:**
- [ ] whisper-rs
- [ ] ort (ONNX Runtime Rust bindings)
- [ ] cpal
- [ ] tokio
- [ ] axum
- [ ] serde
- [ ] rusqlite
- [ ] reqwest

### RQ2: What licenses do native dependencies use?

**Native libraries to audit:**
- [ ] whisper.cpp (via whisper-rs)
- [ ] ONNX Runtime (via ort)
- [ ] SQLite (via rusqlite)
- [ ] OpenSSL or rustls (via reqwest)

### RQ3: What licenses do ML models use?

**Models to audit:**
- [ ] Whisper models (OpenAI)
- [ ] Piper TTS models
- [ ] Any translation models (if used)

### RQ4: What attribution requirements exist?

**Areas to investigate:**
- [ ] In-app attribution requirements
- [ ] Documentation attribution requirements
- [ ] Source code notice requirements
- [ ] Third-party notice file format

---

## Known License Categories

| License Type | Compatibility | Notes |
|--------------|---------------|-------|
| MIT | ✅ Permissive | Attribution only |
| Apache 2.0 | ✅ Permissive | Attribution + patent grant |
| BSD (2/3-clause) | ✅ Permissive | Attribution only |
| ISC | ✅ Permissive | Attribution only |
| MPL 2.0 | ⚠️ Weak copyleft | File-level copyleft |
| LGPL | ⚠️ Weak copyleft | Dynamic linking usually OK |
| GPL | ❌ Strong copyleft | Requires source release |
| AGPL | ❌ Network copyleft | Even stricter than GPL |

---

## Preliminary Dependency Licenses

| Crate | License | Status |
|-------|---------|--------|
| whisper-rs | MIT | ✅ |
| ort | MIT/Apache-2.0 | ✅ |
| cpal | Apache-2.0 | ✅ |
| tokio | MIT | ✅ |
| axum | MIT | ✅ |
| serde | MIT/Apache-2.0 | ✅ |
| rusqlite | MIT | ✅ |
| reqwest | MIT/Apache-2.0 | ✅ |

| Native Lib | License | Status |
|------------|---------|--------|
| whisper.cpp | MIT | ✅ |
| ONNX Runtime | MIT | ✅ |
| SQLite | Public Domain | ✅ |

| Model | License | Status |
|-------|---------|--------|
| Whisper (OpenAI) | MIT | ✅ |
| Piper voices | ? | 🔍 Needs audit |

---

## Agent Tasks

### Task 1: Audit Rust crate licenses
- [ ] Run `cargo license` or `cargo-deny` on project
- [ ] Generate dependency tree with licenses
- [ ] Flag any non-permissive licenses
- [ ] Document transitive dependencies

### Task 2: Audit native library licenses
- [ ] Verify whisper.cpp license
- [ ] Verify ONNX Runtime license
- [ ] Check for any system library dependencies
- [ ] Document build-time vs runtime dependencies

### Task 3: Audit model licenses
- [ ] Research Whisper model license (MIT per OpenAI)
- [ ] Research Piper voice model licenses (varies by voice)
- [ ] Research any other ML models used
- [ ] Document per-model attribution requirements

### Task 4: Create attribution documentation
- [ ] Generate NOTICE or THIRD_PARTY_LICENSES file
- [ ] Document in-app attribution requirements
- [ ] Create license display UI recommendations
- [ ] Document app store compliance requirements

### Task 5: Recommend project license
- [ ] Analyze compatible license options
- [ ] Consider commercial vs open-source goals
- [ ] Draft LICENSE file
- [ ] Document any restrictions

---

## Tools

### cargo-license
```bash
cargo install cargo-license
cargo license
```

### cargo-deny
```bash
cargo install cargo-deny
cargo deny init
cargo deny check licenses
```

### cargo-about
```bash
cargo install cargo-about
cargo about generate about.hbs > THIRD_PARTY_LICENSES.html
```

---

## Success Criteria

- [ ] All Rust dependencies audited with licenses documented
- [ ] All native dependencies audited with licenses documented
- [ ] All model licenses audited and documented
- [ ] No GPL/AGPL dependencies (or documented plan if present)
- [ ] THIRD_PARTY_LICENSES file generated
- [ ] Project license recommendation made

---

## Attribution Template

```
Third-Party Licenses
====================

whisper-rs
----------
License: MIT
Copyright (c) [year] [author]
https://github.com/tazz4843/whisper-rs

whisper.cpp
-----------
License: MIT
Copyright (c) 2023 Georgi Gerganov
https://github.com/ggerganov/whisper.cpp

Whisper Models
--------------
License: MIT
Copyright (c) 2022 OpenAI
https://github.com/openai/whisper

[Continue for all dependencies...]
```

---

## Dependencies

- None (can be done independently)

---

## References

- cargo-deny: https://github.com/EmbarkStudios/cargo-deny
- cargo-license: https://github.com/onur/cargo-license
- cargo-about: https://github.com/EmbarkStudios/cargo-about
- SPDX License List: https://spdx.org/licenses/
- tl;dr Legal: https://tldrlegal.com/
- OpenAI Whisper License: https://github.com/openai/whisper/blob/main/LICENSE
- Piper License: https://github.com/rhasspy/piper/blob/master/LICENSE

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
