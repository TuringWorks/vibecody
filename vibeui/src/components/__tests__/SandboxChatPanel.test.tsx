import { describe, it, expect } from 'vitest';
import { buildSandboxSystemPrompt } from '../SandboxChatPanel';

describe('buildSandboxSystemPrompt', () => {
  const path = '/tmp/sandbox';
  const entries = [
    { path: '/tmp/sandbox/src', name: 'src', is_directory: true },
    { path: '/tmp/sandbox/README.md', name: 'README.md', is_directory: false, size: 42 },
  ];

  it('embeds the sandbox path', () => {
    const out = buildSandboxSystemPrompt(path, entries);
    expect(out).toContain(`Folder: ${path}`);
  });

  it('lists current contents', () => {
    const out = buildSandboxSystemPrompt(path, entries);
    expect(out).toContain('src');
    expect(out).toContain('README.md');
  });

  it('mandates action over analysis (load-bearing wording)', () => {
    const out = buildSandboxSystemPrompt(path, entries);
    // The prompt MUST tell the model to act, not summarise. Without these
    // directives, weak local models respond with bullet-list code reviews
    // instead of using the tool tags.
    expect(out).toMatch(/require ACTION, not analysis/);
    expect(out).toMatch(/Do NOT respond with a summary/);
    expect(out).toMatch(/DO it/);
  });

  it('declares the agent will be re-invoked after each tool call', () => {
    const out = buildSandboxSystemPrompt(path, entries);
    expect(out).toMatch(/re-invoked with the output/);
    expect(out).toMatch(/Continue the task across/);
  });

  it('forbids inventing file contents', () => {
    const out = buildSandboxSystemPrompt(path, entries);
    expect(out).toMatch(/read the relevant files before writing/);
    expect(out).toMatch(/Do not invent file contents/);
  });

  it('shows both tag forms with the sandbox path interpolated', () => {
    const out = buildSandboxSystemPrompt(path, entries);
    expect(out).toContain(`<read_file path="${path}/relative/path" />`);
    expect(out).toContain(`<write_file path="${path}/relative/path">`);
  });

  it('handles an empty folder', () => {
    const out = buildSandboxSystemPrompt(path, []);
    expect(out).toContain('(empty)');
    // Action directives must still be present even when the tree is empty.
    expect(out).toMatch(/require ACTION, not analysis/);
  });
});
