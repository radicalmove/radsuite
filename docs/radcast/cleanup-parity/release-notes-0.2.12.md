RADsuite 0.2.12 restores the Python RADcast rules for removing spoken fillers and improves long-pause shortening.

- Recognises variations of um, uh, ah and erm, with the Python duration, confidence and context checks.
- Uses the small speech model and overlapping analysis windows for aggressive cleanup; protects words such as umbrella and fillers rejected by the selected mode.
- Applies edits within the selected clip with sample-accurate cuts and short crossfades, including pauses at the beginning and end.
- Preserves cancellation during analysis and audio rendering.
- Includes guarded Studio and Studio Treble processing, retained quality reports, and silence analysis from the local 0.2.11 work.
- Preserves the current MP4 exports and installer improvements. Studio quality reports verify the delivered MP4 as well as audio exports.

The speech recognition engine remains whisper.cpp, so recognition can differ from the original Python faster-whisper version.
