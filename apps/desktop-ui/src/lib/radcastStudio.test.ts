import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

describe('Studio experiment UI compatibility', () => {
  const workspace = readFileSync(new URL('../components/RadcastWorkspace.svelte', import.meta.url), 'utf8');
  it('registers opt-in Studio and keeps the existing initial default', () => {
    expect(workspace).toContain('id: "studio_v1"');
    expect(workspace).toContain('label: "Studio — Classic"');
    expect(workspace).toContain('$state<EnhancementModel>("studio_v18")');
  });
  it('retains legacy selection IDs and displays QA reports', () => {
    expect(workspace).toContain('id: "studio_v18"');
    expect(workspace).toContain('id: "studio_v18_natural_double_plus"');
    expect(workspace).toContain('output.studio_qa_path');
  });
  it('offers adaptive natural voice without changing the saved initial default', () => {
    expect(workspace).toContain('id: "studio_treble"');
    expect(workspace).toContain('label: "Studio — Treble (recommended)"');
    expect(workspace).toContain('$state<EnhancementModel>("studio_v18")');
  });
});
