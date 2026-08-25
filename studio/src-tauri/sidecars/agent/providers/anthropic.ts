/**
 * Claude provider, on the official Anthropic SDK.
 *
 * This is the default path. Notes that are easy to get wrong on Claude Opus 5:
 *
 *   - `temperature` / `top_p` / `top_k` are REMOVED and return 400. Steer with
 *     the prompt instead.
 *   - Thinking is ON by default; `{type: "enabled", budget_tokens: N}` is gone
 *     and returns 400. `max_tokens` caps thinking + response text together.
 *   - Thinking blocks must be replayed back UNCHANGED on the same model, so
 *     the assistant turn is recorded verbatim rather than reconstructed.
 *   - Safety classifiers can decline with `stop_reason: "refusal"` on an
 *     HTTP 200, with empty or partial content. Reading content[0] blindly
 *     breaks. Server-side fallbacks are opted into by default so a decline is
 *     re-run on a fallback model rather than surfacing as a dead turn.
 */

import Anthropic from '@anthropic-ai/sdk';
import type { Provider, ToolSpec, ToolCall, ToolOutcome, TurnResult } from './types.js';

/** Default model. Override with ONTO_LLM_MODEL. */
const DEFAULT_MODEL = 'claude-opus-5';

/**
 * Server-side refusal fallbacks are Claude-API-only, not available on the
 * third-party hosting platforms that mirror this SDK's interface. Enabled by
 * default, disable with ONTO_ANTHROPIC_FALLBACKS=0 when
 * pointing at a non-first-party endpoint.
 */
const FALLBACK_BETA = 'server-side-fallback-2026-07-01';

export class AnthropicProvider implements Provider {
  readonly name = 'anthropic';

  private client: Anthropic;
  private model: string;
  private system: string;
  private maxTokens: number;
  private useFallbacks: boolean;
  // Beta param types are a superset of the stable ones, so using them for
  // history keeps one representation whether or not fallbacks are enabled.
  private messages: Anthropic.Beta.BetaMessageParam[] = [];

  constructor(opts: { system: string; model?: string; apiKey?: string; baseURL?: string }) {
    this.system = opts.system;
    this.model = opts.model || process.env.ONTO_LLM_MODEL || DEFAULT_MODEL;
    this.maxTokens = Number(process.env.ONTO_MAX_OUTPUT_TOKENS ?? 16000);
    this.useFallbacks = process.env.ONTO_ANTHROPIC_FALLBACKS !== '0';

    // A bare constructor resolves ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, or
    // an `ant auth login` profile. Only pass a key when one was given
    // explicitly, so profile-based auth keeps working.
    this.client = new Anthropic({
      ...(opts.apiKey ? { apiKey: opts.apiKey } : {}),
      ...(opts.baseURL ? { baseURL: opts.baseURL } : {}),
    });
  }

  async describe(): Promise<string> {
    return this.model;
  }

  setSystem(system: string): void {
    this.system = system;
  }

  reset(): void {
    this.messages = [];
  }

  addUser(text: string): void {
    this.messages.push({ role: 'user', content: text });
  }

  async step(tools: ToolSpec[]): Promise<TurnResult> {
    // MCP input schemas are already JSON Schema, so they map straight across.
    const anthropicTools: Anthropic.Beta.BetaTool[] = tools.map(t => ({
      name: t.name,
      description: t.description.slice(0, 1024),
      input_schema: t.inputSchema as Anthropic.Beta.BetaTool.InputSchema,
    }));

    const params = {
      model: this.model,
      max_tokens: this.maxTokens,
      system: this.system,
      tools: anthropicTools,
      messages: this.messages,
      // No temperature/top_p/top_k: removed on Opus 5, they return 400.
      // No thinking config: adaptive is the default on Opus 5.
    };

    // Always the beta endpoint: it accepts the same request without beta
    // flags, which avoids maintaining two divergent request shapes.
    const response = await this.client.beta.messages.create({
      ...params,
      ...(this.useFallbacks ? { betas: [FALLBACK_BETA], fallbacks: 'default' } : {}),
    } as Anthropic.Beta.MessageCreateParamsNonStreaming);

    // Record the assistant turn verbatim. Thinking blocks must be replayed
    // unchanged, so never rebuild this from extracted text.
    this.messages.push({ role: 'assistant', content: response.content });

    const text: string[] = [];
    const toolCalls: ToolCall[] = [];

    for (const block of response.content) {
      if (block.type === 'text' && block.text) {
        text.push(block.text);
      } else if (block.type === 'tool_use') {
        toolCalls.push({
          id: block.id,
          name: block.name,
          input: (block.input ?? {}) as Record<string, unknown>,
        });
      }
    }

    return {
      text,
      toolCalls,
      stopReason: response.stop_reason ?? 'end_turn',
    };
  }

  addToolResults(outcomes: ToolOutcome[]): void {
    // All results for a turn go back in ONE user message. Splitting them
    // across messages trains the model out of parallel tool calls.
    const content: Anthropic.Beta.BetaToolResultBlockParam[] = outcomes.map(o => ({
      type: 'tool_result',
      tool_use_id: o.call.id,
      content: o.content,
      ...(o.isError ? { is_error: true } : {}),
    }));
    this.messages.push({ role: 'user', content });
  }
}
