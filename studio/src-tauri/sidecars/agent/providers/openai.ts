/**
 * OpenAI-compatible provider.
 *
 * Deliberately dependency-free (plain fetch) so it works against any endpoint
 * exposing /v1/chat/completions: Azure OpenAI, a cloud provider's
 * OpenAI-compatible gateway, vLLM, Ollama, LM Studio, or a local MLX server.
 *
 * This file must NOT import the Anthropic SDK. Keeping the two providers in
 * separate modules is what makes "swap the model" a config change rather than
 * a rewrite.
 */

import type { Provider, ToolSpec, ToolCall, ToolOutcome, TurnResult } from './types.js';

interface ChatMessage {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string | null;
  tool_calls?: Array<{ id: string; type: 'function'; function: { name: string; arguments: string } }>;
  tool_call_id?: string;
}

export class OpenAICompatibleProvider implements Provider {
  readonly name = 'openai-compatible';

  private baseURL: string;
  private apiKey: string;
  private model: string;
  private system: string;
  private maxTokens: number;
  private messages: ChatMessage[] = [];

  constructor(opts: { system: string; baseURL: string; apiKey?: string; model?: string }) {
    this.system = opts.system;
    this.baseURL = opts.baseURL.replace(/\/$/, '');
    this.apiKey = opts.apiKey ?? 'not-needed';
    this.model = opts.model ?? '';
    this.maxTokens = Number(process.env.ONTO_MAX_OUTPUT_TOKENS ?? 4096);
  }

  /** Resolve a usable model id. MLX servers 404 on a mismatched id. */
  private async resolveModel(): Promise<string> {
    if (this.model) return this.model;
    try {
      const resp = await fetch(`${this.baseURL}/models`, {
        headers: { Authorization: `Bearer ${this.apiKey}` },
      });
      const json: any = await resp.json();
      const first = json?.data?.[0]?.id;
      if (first) {
        this.model = first;
        return first;
      }
    } catch {
      /* fall through */
    }
    this.model = 'default';
    return this.model;
  }

  async describe(): Promise<string> {
    return this.resolveModel();
  }

  setSystem(system: string): void {
    this.system = system;
  }

  reset(): void {
    this.messages = [];
  }

  addUser(text: string): void {
    if (this.messages.length === 0) {
      this.messages.push({ role: 'system', content: this.system });
    }
    this.messages.push({ role: 'user', content: text });
  }

  async step(tools: ToolSpec[]): Promise<TurnResult> {
    const model = await this.resolveModel();

    const resp = await fetch(`${this.baseURL}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${this.apiKey}`,
      },
      body: JSON.stringify({
        model,
        messages: this.messages,
        tools: tools.map(t => ({
          type: 'function' as const,
          function: {
            name: t.name,
            description: t.description.slice(0, 1024),
            parameters: t.inputSchema,
          },
        })),
        tool_choice: 'auto',
        temperature: 0.7,
        top_p: 0.8,
        max_tokens: this.maxTokens,
        // Qwen-family servers only honour thinking suppression through this
        // key. Ignored by other providers.
        chat_template_kwargs: { enable_thinking: false },
      }),
    });

    if (!resp.ok) {
      throw new Error(`Model endpoint ${this.baseURL} returned HTTP ${resp.status}: ${await resp.text()}`);
    }

    const completion: any = await resp.json();
    const message = completion?.choices?.[0]?.message;
    if (!message) throw new Error('Model returned no message');

    const rawCalls = message.tool_calls ?? [];

    this.messages.push({
      role: 'assistant',
      content: message.content ?? null,
      ...(rawCalls.length ? { tool_calls: rawCalls } : {}),
    });

    const toolCalls: ToolCall[] = rawCalls.map((c: any) => {
      let input: Record<string, unknown> = {};
      try {
        input = c.function?.arguments ? JSON.parse(c.function.arguments) : {};
      } catch {
        // Some models emit not-quite-JSON arguments. Surface it as an empty
        // input rather than crashing the turn.
        input = {};
      }
      return { id: c.id, name: c.function?.name ?? '', input };
    });

    return {
      text: message.content ? [message.content] : [],
      toolCalls,
      stopReason: toolCalls.length > 0 ? 'tool_use' : 'end_turn',
    };
  }

  addToolResults(outcomes: ToolOutcome[]): void {
    for (const o of outcomes) {
      this.messages.push({
        role: 'tool',
        tool_call_id: o.call.id,
        content: o.content,
      });
    }
  }
}
