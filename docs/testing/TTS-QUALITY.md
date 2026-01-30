# TTS Quality Baseline

> **Evaluation Date**: 2026-01-25
>
> **Model**: Kokoro v0.19 with misaki-rs G2P
>
> **Status**: Shipped as known limitation

---

## Summary

Kokoro TTS produces functional, understandable speech suitable for general text-to-speech use cases. Quality is clearly synthetic but sufficient for the Beta release.

---

## Evaluation Results

| Metric | Score | Target | Status |
|--------|-------|--------|--------|
| **Average MOS** | **3.0** | 3.5 | Accepted |
| Intelligibility | Good | 4.0 | Acceptable |
| Naturalness | Fair | 3.5 | Known limitation |
| Pronunciation | Good | 4.5 | Acceptable |
| Prosody | Fair | 3.5 | Known limitation |

### MOS Scale Reference

| Score | Label | Description |
|-------|-------|-------------|
| 5 | Excellent | Indistinguishable from human speech |
| 4 | Good | Natural with only minor issues |
| **3** | **Fair** | **Clearly synthetic but understandable** |
| 2 | Poor | Some difficulty understanding |
| 1 | Bad | Unintelligible |

---

## Known Limitations

### Naturalness
- Speech has a clearly synthetic quality
- Rhythm can feel mechanical in longer sentences
- Emotional expression is limited

### Prosody
- Questions may not always have natural rising intonation
- Emphasis patterns can be inconsistent
- Pauses between clauses may feel unnatural

### Pronunciation
- Most common words pronounced correctly
- Some technical terms may be mispronounced
- Abbreviations generally handled well (Dr., Mr., USA, etc.)

---

## Acceptable Use Cases

- General text-to-speech for accessibility
- Reading notifications and alerts
- Reviewing transcriptions audibly
- Prototyping and development

## Not Recommended For

- Public-facing voice assistants requiring human-like quality
- Audiobook narration
- Customer service voice responses

---

## Future Improvements

TTS quality improvement is deferred to the Release phase. Options include:

1. **Voice/speaker tuning** — Explore Kokoro's voice configuration options
2. **Alternative models** — Evaluate other TTS engines (see R-TTS-01)
3. **Post-processing** — Apply audio enhancement to improve naturalness

---

## Test Sentences

The following sentences were used for evaluation:

1. "Hello, how are you today?"
2. "The quick brown fox jumps over the lazy dog."
3. "The meeting is scheduled for January 15th, 2026 at 3:30 PM."
4. "The price is $1,234.56 including tax."
5. "Call me at 555-123-4567."
6. "Dr. Smith works at MIT in the USA."
7. "The CEO of IBM met with the FBI director."
8. "Wait... what did you say?"
9. "This is amazing! I can't believe it!"
10. "Should we go left, right, or straight ahead?"
11. "The API returns JSON data over HTTPS."
12. "Configure the WiFi settings in the iOS app."
13. "In order to successfully complete this task, you will need to carefully follow each step of the instructions provided in the documentation."
14. "The entrepreneur scheduled a rendezvous at the boutique."
15. "The colonel analyzed the thoroughly sophisticated algorithm."

---

*Last updated: 2026-01-25*
