# Single bounded preservation repair

The first full Finnegan trial is rejected, with `quiet_speech_loss` under the unchanged watchdog. The final worst eligible window is −23.08 dB relative to median gain. It remains named `finnegan-primary/master_attempt.wav`; it is diagnostic evidence, not a listening candidate.

The saved stage inspection localizes the loss to the joint model: highpass worst relative gain is −0.20 dB, raw model −27.81 dB. Pause control does not change the flagged windows. The final flagged contexts start at 4, 4.5, 138, 138.5, 139 and 140 seconds. The periodicity/modulation detector cannot establish whether this very faint content is speech, background voices or room signal. Its protection remains active; no threshold or eligibility change is justified.

Use the plan's one permitted repair: 80% saved-model-equivalent output plus 20% highpassed original, across the full band and full recording, before the identical downstream policy. No second enhancement stage, EQ change, segmented selection or gain boost is added. The adapter re-runs the same pinned model deterministically and records that the explicit inference blend is 0.20; this must not be confused with the model's training target mixture.

The weight is chosen from the observed attenuation, not a trial matrix. Raw worst-window amplitude gain is about 0.0246 and typical gain about 0.605. A triangle lower bound for the blended worst-window gain is 0.20 − 0.80 × 0.0246 ≈ 0.1803, compared with an approximate typical gain of 0.684. This is approximately −11.6 dB relative retention before downstream processing. Phase and adaptive leveling mean that this calculation does not guarantee final QA; the unchanged full-source check decides acceptance.

Original blending also restores some room sound. A passing repair therefore does not establish useful dereverberation or Adobe parity. Stop if this one repair rejects. If it passes, offer only that candidate for matched listening against B and Adobe, including the repaired faint contexts. No further strength escalation is authorized by this experiment.

[Per-stage evidence](/Users/rcd58/Documents/RADsuite/docs/radcast/studio-experiment/model-feasibility/treble-ism-eng120-epoch119/preservation-diagnosis.json).
