# Native clean-speech control source

9 October 2026. Obtained original clean WAV members from the University of Edinburgh release using HTTP byte ranges. No resampling, trimming, concatenation, gain change or enhancement was performed.

## Attribution and licence

Valentini-Botinhao, Cassia. (2017). Noisy speech database for training speech enhancement algorithms and TTS models, 2016 [sound]. University of Edinburgh. School of Informatics. Centre for Speech Technology Research (CSTR). https://doi.org/10.7488/ds/2117.

Source: [original Edinburgh item](https://datashare.ed.ac.uk/handle/10283/2791) · [DOI](https://doi.org/10.7488/ds/2117) · [author API metadata](https://datashare.ed.ac.uk/server/api/core/items/6ed35425-bf14-4d2b-93a1-0a4984952757).

The item metadata explicitly declares Creative Commons Attribution 4.0. The repository’s original CC-LICENSE bitstream independently contains the [CC BY 4.0 licence](https://creativecommons.org/licenses/by/4.0/). Its original licence text and metadata snapshots are retained under `controls-source/`; snapshot hashes are recorded in the JSON manifest. Credit the author and source, link the licence, and identify any later transformations. These extracted members are unchanged.

## Verified selected audio

Both test speakers are represented: p232 has **25.206 s** in five utterances; p257 has **25.184 s** in six. Every WAV header verifies **48,000 Hz, mono, 16-bit PCM**. The original release metadata describes a database designed for 48 kHz enhancement; native rate was independently checked rather than inferred from that description.

The official text archive supplies spoken-sentence content including voiced vowels and consonant-rich phrases such as “Six spoons of fresh snow peas, five thick slabs of blue cheese” and “sunlight strikes raindrops.” This confirms meaningful speech content; no listening/articulation-quality judgment or anechoic-room ground truth is claimed.

| File | Duration s | SHA256 |
|---|---:|---|
| [p232_002.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p232_002.wav) | 2.715208 | `034de7f89e16082e6aeae23746978d2a42add1d81c29d9f8c615b3e020d70643` |
| [p232_003.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p232_003.wav) | 7.184854 | `85bd30db189b8abdde6719efb07b34965fb5fb9b6e476ae23f6e6b2d4fd720db` |
| [p232_005.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p232_005.wav) | 6.246625 | `89b1ebdeabff150a3cc12629178e5b98a2366e052f3e8028ea853c2059f69494` |
| [p232_006.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p232_006.wav) | 5.103500 | `337d07d33650b223ec973413db1e455aa907dc21a94e49f0544f561e2bce600c` |
| [p232_007.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p232_007.wav) | 3.955896 | `74d3ecad4aacbb89cfc4d1bd6dde9bb2c12f6014aaced6e281ff961c421f48f7` |
| [p257_002.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p257_002.wav) | 2.776125 | `b9608fa621bf688a9c8f40fd6e6bf225ef5e06823b806eab3836f2d3a941ca92` |
| [p257_003.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p257_003.wav) | 5.521417 | `394acaaf0d1bf55678ed10e4ce5f09cc691dcd4e0e99d0d66809637dbb37e197` |
| [p257_004.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p257_004.wav) | 3.631417 | `d2bb1c605d416df5fafa2aea4d4fb83df78bd3f9a46260606f5e3d1b58042f2e` |
| [p257_006.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p257_006.wav) | 4.274500 | `336051b3e1eb146169c48d23e56ef4abae990c22f223fe11a43cec896419024e` |
| [p257_007.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p257_007.wav) | 3.397667 | `422f5f0d43ad411d811188e2dc4dfafe8784ab793759f22dd3ee784089976587` |
| [p257_008.wav](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/controls-source/wav/p257_008.wav) | 5.583104 | `e4d93c15e49a577b7db5c58cec7e02a18d18bdf6c4896fc250c3588849d64c20` |

## Transfer and integrity

Original archive: `clean_testset_wav.zip`, declared size **154,332,064 bytes**, original bitstream `dec213d3-bf57-4777-9663-c24bdce92d5e`. [Author content endpoint](https://datashare.ed.ac.uk/server/api/core/bitstreams/dec213d3-bf57-4777-9663-c24bdce92d5e/content).

HTTP 206 responses supplied the final 262,144 archive bytes (including the ZIP central directory) and exactly the ranges needed for selected members. Each local-header/member name, decompressed byte count and ZIP CRC32 was checked. Every retained range and extracted member has its own SHA256, with offsets in [the manifest](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/clean-control-source.json).

The entire archive was not downloaded. Its author-reported MD5 is recorded as metadata only, not verified; no whole-archive SHA256 is invented. The official transcript archive was downloaded completely (440,277 bytes), and its author MD5 was verified. Only selected transcripts were extracted.

Retained source-artifact response bodies total **4,645,355 bytes**, approximately 4.65 MB; small metadata/header responses are additional. Total transfer is below 5 MB and the 160 MB cap. No noisy or training archive was requested.

## Handoff

Use individual source WAVs from `controls-source/wav/` for known-clean and declared synthetic-reflection controls. Preserve these originals and their hashes; write transformed controls and model outputs in the parent-owned experiment folder. This is licensed known-clean speech, not proof that Finnegan or Adobe has the same room/source characteristics. No model, package or runtime was changed.
