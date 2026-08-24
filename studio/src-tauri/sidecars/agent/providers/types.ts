/**
 * Provider abstraction for the ontology agent.
 *
 * The engine has no model client in it. The orchestration layer (this sidecar)
 * does, and it is deliberately pluggable: Claude is the default and a
 * first-class path, and any OpenAI-compatible endpoint is an equal citizen.
 *
 * Each provider owns its own conversation history in its own wire format.
 * Nothing above this interface knows what a "content block" or a
 * "tool_call" looks like on the wire, which is what keeps the agent loop
 * provider-agnostic instead of provider-shaped.
 */

/** An MCP tool, normalised away from MCP's own envelope. */
export interface ToolSpec {
  name: string;
  description: string;
  /** JSON Schema for the tool's arguments. */
  inputSchema: Record<string, unknown>;
}

/** A tool invocation requested by the model. */
export interface ToolCall {
  /** Provider-assigned id, echoed back with the result. */
  id: string;
  name: string;
  input: Record<string, unknown>;
}

/** The result of executing one tool call. */
export interface ToolOutcome {
  call: ToolCall;
  content: string;
  isError: boolean;
}

/** What one assistant turn produced. */
export interface TurnResult {
  /** Assistant text emitted this turn, in order. */
  text: string[];
  /** Tool calls the model wants executed before it continues. */
  toolCalls: ToolCall[];
  /**
   * Why the model stopped. Providers normalise to these values:
   *   "tool_use"  - wants tools run, then continue
   *   "end_turn"  - finished
   *   "refusal"   - declined on policy grounds; text may be empty
   *   "max_tokens"- hit the output cap
   *   "pause_turn"- server-side tool loop paused; resend to continue
   */
  stopReason: string;
}

export interface Provider {
  /** Short identifier for logs and the UI banner, e.g. "anthropic". */
  readonly name: string;

  /** Human-readable description of the active endpoint and model. */
  describe(): Promise<string>;

  /** Clear conversation history. */
  reset(): void;

  /**
   * Replace the system prompt.
   *
   * The sidecar does two different jobs with one provider instance: it drives
   * a tool-using agent, and it answers questions from retrieved context with
   * no tools at all. Those need different system prompts, and previously the
   * question-answering path prepended its own instructions to a user message
   * while the agent's system prompt stayed in place underneath. The model was
   * told it had MCP tools and should call onto_load, then asked a question it
   * had to answer from supplied text. Setting the system prompt for the job
   * removes that collision.
   */
  setSystem(system: string): void;

  /** Append a user message. */
  addUser(text: string): void;

  /**
   * Send the current history and record the assistant turn into history.
   * Implementations MUST record the turn verbatim in their own wire format:
   * Anthropic thinking blocks in particular have to be replayed unchanged.
   */
  step(tools: ToolSpec[]): Promise<TurnResult>;

  /** Append tool results for the calls returned by the last `step`. */
  addToolResults(outcomes: ToolOutcome[]): void;
}
