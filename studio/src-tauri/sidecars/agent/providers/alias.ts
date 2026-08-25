/**
 * Provider selection.
 */

import type { Provider } from './types.js';
import { AnthropicProvider } from './anthropic.js';
import { OpenAICompatibleProvider } from './openai.js';
import { ClaudeCliProvider } from './claudecli.js';

/**
 * Select the provider.
 *
 * ONTO_PROVIDER=anthropic|openai|claude-cli forces one. Left unset, an
 * explicitly configured OpenAI-compatible base URL selects that path, a
 * present ANTHROPIC_API_KEY selects Claude, and otherwise the sidecar falls
 * back to a local OpenAI-compatible endpoint. That local fallback matters in
 * practice: an app launched from Finder has no shell environment, so an
 * unconditional Claude default died with an authentication error before the
 * first token, which is what this chain avoids.
 *
 * Takes the system prompt alongside `env` because every provider constructor
 * needs one at construction time; `Provider` itself lets it be replaced later
 * via `setSystem`.
 */
export function selectProvider(env: NodeJS.ProcessEnv, system: string): Provider {
  const explicit = (env.ONTO_PROVIDER ?? '').toLowerCase();
  const baseURL = env.ONTO_LLM_BASE_URL;

  if (explicit === 'openai' || explicit === 'openai-compatible') {
    return new OpenAICompatibleProvider({
      system,
      baseURL: baseURL ?? 'http://localhost:8081/v1',
      apiKey: env.ONTO_LLM_API_KEY,
      model: env.ONTO_LLM_MODEL,
    });
  }
  if (explicit === 'claude-cli' || explicit === 'cli') {
    return new ClaudeCliProvider({ system });
  }
  if (explicit === 'anthropic' || explicit === 'claude') {
    return new AnthropicProvider({ system, model: env.ONTO_LLM_MODEL });
  }
  if (baseURL) {
    return new OpenAICompatibleProvider({
      system,
      baseURL,
      apiKey: env.ONTO_LLM_API_KEY,
      model: env.ONTO_LLM_MODEL,
    });
  }
  if (env.ANTHROPIC_API_KEY) {
    return new AnthropicProvider({ system, model: env.ONTO_LLM_MODEL });
  }
  return new OpenAICompatibleProvider({
    system,
    baseURL: 'http://localhost:8081/v1',
    apiKey: env.ONTO_LLM_API_KEY,
    model: env.ONTO_LLM_MODEL,
  });
}
