/**
 * Provider that drives the locally installed `claude` CLI in print mode.
 *
 * Why this exists: it uses an already-authenticated CLI, so there is no API
 * key to provision, rotate or leak into a demo environment. For the GraphRAG
 * path that is sufficient, because retrieval happens BEFORE generation: the
 * subgraph is assembled by the engine and handed over as context, so the model
 * only has to write the answer. It never needs to call a tool.
 *
 * The trade is explicit: this provider does not support tool use. Selecting it
 * for the ontology-authoring workflows, which are tool-driven, will not work.
 * It is for question answering.
 */

import { spawn } from 'child_process';
import type { Provider, ToolSpec, ToolOutcome, TurnResult } from './types.js';
import { displayModel } from './alias.js';

export class ClaudeCliProvider implements Provider {
  readonly name = 'claude-cli';

  private bin: string;
  private model?: string;
  private system: string;
  private history: string[] = [];

  constructor(opts: { system: string; model?: string; bin?: string }) {
    this.system = opts.system;
    this.model = opts.model ?? process.env.ONTO_CLAUDE_CLI_MODEL;
    this.bin = opts.bin ?? process.env.ONTO_CLAUDE_BIN ?? 'claude';
  }

  async describe(): Promise<string> {
    return displayModel(this.model ?? 'claude-cli');
  }

  setSystem(system: string): void {
    this.system = system;
  }

  reset(): void {
    this.history = [];
  }

  addUser(text: string): void {
    this.history.push(text);
  }

  async step(_tools: ToolSpec[]): Promise<TurnResult> {
    // Only the latest turn is sent: the retrieved context and the question are
    // self-contained, and the system prompt travels as a flag rather than as a
    // prefix on the user message.
    const prompt = this.history[this.history.length - 1] ?? '';

    // `--system-prompt` REPLACES the CLI's own prompt. Without it the CLI
    // loads its default agent prompt plus the user's and project's CLAUDE.md
    // files and any session hooks, and those leak into the answer: a question
    // about a dataset's licence conformance came back opening with this
    // machine's agent routing mode instead of the answer. Replacing the
    // prompt, dropping the per-machine sections and disabling session
    // persistence makes each call a clean, stateless completion that depends
    // only on what this sidecar supplies.
    const args = [
      '-p',
      '--system-prompt', this.system,
      '--exclude-dynamic-system-prompt-sections',
      '--no-session-persistence',
    ];
    if (this.model) args.push('--model', this.model);

    const text = await new Promise<string>((resolve, reject) => {
      const child = spawn(this.bin, args, { stdio: ['pipe', 'pipe', 'pipe'] });
      let out = '';
      let err = '';
      const timer = setTimeout(() => {
        child.kill('SIGKILL');
        reject(new Error('claude CLI timed out'));
      }, Number(process.env.ONTO_CLAUDE_TIMEOUT_MS ?? 300000));

      child.stdout.on('data', d => (out += d.toString()));
      child.stderr.on('data', d => (err += d.toString()));
      child.on('error', e => {
        clearTimeout(timer);
        reject(new Error(`could not run "${this.bin}": ${e}`));
      });
      child.on('close', code => {
        clearTimeout(timer);
        if (code !== 0 && !out.trim()) {
          reject(new Error(`claude CLI exited ${code}: ${err.slice(0, 400)}`));
        } else {
          resolve(out.trim());
        }
      });

      child.stdin.write(prompt);
      child.stdin.end();
    });

    const cleaned = stripHostPreamble(text);
    this.history.push(cleaned);
    return { text: [cleaned], toolCalls: [], stopReason: 'end_turn' };
  }

  addToolResults(_outcomes: ToolOutcome[]): void {
    // No tool use on this path. Retrieval is done before generation.
  }
}

/**
 * Remove host-environment preamble from a CLI reply.
 *
 * The `claude` CLI runs inside the operator's own configuration. Session hooks
 * and memory files can prepend a paragraph that has nothing to do with the
 * question: one run answering a question about a dataset's distribution came
 * back opening with this machine's agent routing mode, and another with a
 * note about which skill applied. `--system-prompt`,
 * `--exclude-dynamic-system-prompt-sections` and `--settings` all failed to
 * suppress it reliably; it appears intermittently, which is the worst kind of
 * defect to carry into a live demonstration.
 *
 * This strips a LEADING paragraph that is recognisably about the host rather
 * than the question. It is conservative on purpose: it only ever removes the
 * first block, only when it matches a known host pattern, and never when that
 * would leave nothing behind. Anything it fails to catch is still visible
 * rather than silently mangled.
 *
 * The durable fix is to talk to the API directly instead of through a CLI that
 * carries an operator's environment. This keeps the demo safe until then.
 */
const HOST_PATTERNS = [
  /routing mode/i,
  /\bqwen\b/i,
  /\bkimi\b/i,
  /no skill applies/i,
  /skill (?:applies|is relevant)/i,
  /^(?:let me|i'?ll) answer (?:from|the)/i,
  /not an ontology-engineering task/i,
  /^now,? to your question/i,
  /^(?:this|that) is a graph question/i,
  /so i'?ll (?:answer|lead) /i,
];

export function stripHostPreamble(text: string): string {
  const blocks = text.split(/\n\s*\n/);
  if (blocks.length < 2) return text.trim();

  const first = blocks[0].trim();
  const looksLikeHost = HOST_PATTERNS.some(re => re.test(first));
  if (!looksLikeHost) return text.trim();

  const rest = blocks.slice(1).join('\n\n').trim();
  return rest.length > 0 ? rest : text.trim();
}
