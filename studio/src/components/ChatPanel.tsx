import { useState, useRef, useEffect, useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { useChat, ChatMessage, BuildMode } from '../hooks/useChat';

const SLASH_COMMANDS = [
  { cmd: '/build', description: 'Deep build (IES-level)', prompt: 'Build an ontology about ' },
  { cmd: '/sketch', description: 'Quick sketch (prototype)', prompt: 'Sketch an ontology about ' },
  { cmd: '/expand', description: 'Expand current ontology', prompt: 'Expand the current ontology with more detail about ' },
  { cmd: '/validate', description: 'Run full validation', prompt: 'Run onto_validate and onto_lint on the current ontology and report all issues' },
  { cmd: '/reason', description: 'Run OWL reasoning', prompt: 'Run onto_reason with profile owl-rl on the current ontology' },
  { cmd: '/enforce', description: 'Check design patterns', prompt: 'Run onto_enforce with the generic rule pack' },
  { cmd: '/query', description: 'Run SPARQL query', prompt: 'Run this SPARQL query: ' },
  { cmd: '/stats', description: 'Show statistics', prompt: 'Run onto_stats and summarize the current ontology' },
  { cmd: '/save', description: 'Export ontology', prompt: 'Save the current ontology to a Turtle file' },
];

const STARTER_CHIPS = [
  { label: 'Build an ontology', prompt: 'Build an ontology about ' },
  { label: 'Expand current', prompt: 'Expand the current ontology with more detail about ' },
  { label: 'Validate', prompt: 'Validate the current ontology and report any issues' },
  { label: 'Run reasoning', prompt: 'Run OWL reasoning on the current ontology' },
  { label: 'Explain structure', prompt: 'Explain the structure of the current ontology' },
  { label: 'Ingest data', prompt: 'Ingest data from ' },
];

export function ChatPanel() {
  const { messages, isTyping, sendMessage, reset, mode, setMode, progress } = useChat();
  const [input, setInput] = useState('');
  const [showSlash, setShowSlash] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const filteredCommands = useMemo(
    () => SLASH_COMMANDS.filter(c => input.startsWith('/') ? c.cmd.startsWith(input) : false),
    [input]
  );

  useEffect(() => {
    setShowSlash(input.startsWith('/') && filteredCommands.length > 0);
  }, [input, filteredCommands.length]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [messages, isTyping]);

  const handleSend = () => {
    const text = input.trim();
    if (!text) return;
    setInput('');
    sendMessage(text);
  };

  const showStarters = messages.length <= 1;

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-2 border-b"
           style={{ borderColor: 'var(--border)' }}>
        <span className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>Chat</span>
        <div className="flex items-center gap-2">
          <div className="flex items-center rounded-full text-xs"
               style={{ background: 'var(--bg-panel)', border: '1px solid var(--border)' }}>
            {(['sketch', 'build'] as BuildMode[]).map((m) => (
              <button key={m}
                onClick={() => setMode(m)}
                className="px-2.5 py-0.5 rounded-full capitalize transition-colors"
                style={{
                  background: mode === m ? 'var(--accent)' : 'transparent',
                  color: mode === m ? 'var(--bg-primary)' : 'var(--text-secondary)',
                }}>
                {m}
              </button>
            ))}
          </div>
          <button onClick={reset} className="text-xs px-2 py-0.5 rounded"
                  style={{ color: 'var(--text-secondary)', background: 'var(--bg-panel)' }}>
            Reset
          </button>
        </div>
      </div>

      <div ref={scrollRef} className="flex-1 overflow-y-auto p-3 space-y-3">
        {showStarters && (
          <div className="space-y-2">
            <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
              What would you like to do?
            </p>
            <div className="flex flex-wrap gap-2">
              {STARTER_CHIPS.map((chip) => (
                <button key={chip.label}
                  onClick={() => setInput(chip.prompt)}
                  className="text-xs px-3 py-1.5 rounded-full border transition-colors hover:opacity-80"
                  style={{ borderColor: 'var(--border)', color: 'var(--accent)', background: 'var(--bg-panel)' }}>
                  {chip.label}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}

        {progress && (
          <div className="px-3 py-2 space-y-1">
            <div className="flex justify-between text-xs" style={{ color: 'var(--text-secondary)' }}>
              <span>{progress.label}</span>
              <span>{Math.round((progress.step / progress.total) * 100)}%</span>
            </div>
            <div className="w-full h-1.5 rounded-full overflow-hidden" style={{ background: 'var(--bg-panel)' }}>
              <div className="h-full rounded-full transition-all duration-500"
                   style={{ width: `${(progress.step / progress.total) * 100}%`, background: 'var(--accent)' }} />
            </div>
          </div>
        )}

        {isTyping && !progress && (
          <div className="flex gap-1 px-3 py-2" style={{ color: 'var(--text-secondary)' }}>
            <span className="animate-pulse">.</span>
            <span className="animate-pulse" style={{ animationDelay: '0.2s' }}>.</span>
            <span className="animate-pulse" style={{ animationDelay: '0.4s' }}>.</span>
          </div>
        )}
      </div>

      {showSlash && (
        <div className="mx-3 mb-1 rounded overflow-hidden"
             style={{ background: 'var(--bg-panel)', border: '1px solid var(--border)' }}>
          {filteredCommands.map((cmd) => (
            <button key={cmd.cmd}
              onClick={() => { setInput(cmd.prompt); setShowSlash(false); }}
              className="w-full text-left px-3 py-1.5 text-xs flex justify-between hover:opacity-80"
              style={{ color: 'var(--text-primary)' }}>
              <span style={{ color: 'var(--accent)' }}>{cmd.cmd}</span>
              <span style={{ color: 'var(--text-secondary)' }}>{cmd.description}</span>
            </button>
          ))}
        </div>
      )}

      <div className="p-3 border-t" style={{ borderColor: 'var(--border)' }}>
        <div className="flex gap-2">
          <input value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend(); } }}
            placeholder="Ask about ontologies..."
            className="flex-1 px-3 py-2 text-sm rounded outline-none"
            style={{ background: 'var(--bg-panel)', color: 'var(--text-primary)', border: '1px solid var(--border)' }} />
          <button onClick={handleSend}
            disabled={!input.trim() || isTyping}
            className="px-3 py-2 text-sm rounded font-medium"
            style={{ background: input.trim() && !isTyping ? 'var(--accent)' : 'var(--bg-panel)',
                     color: input.trim() && !isTyping ? 'var(--bg-primary)' : 'var(--text-secondary)' }}>
            Send
          </button>
        </div>
      </div>
    </div>
  );
}

function MessageBubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div className="max-w-[85%] px-3 py-2 rounded-lg text-sm"
        style={{
          background: isUser ? 'var(--accent)' : 'var(--bg-panel)',
          color: isUser ? 'var(--bg-primary)' : 'var(--text-primary)',
          opacity: isSystem ? 0.7 : 1,
        }}>
        {isUser || isSystem
          ? <div className="whitespace-pre-wrap">{message.content}</div>
          : <div className="prose prose-sm max-w-none markdown-body">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content}</ReactMarkdown>
            </div>
        }
        {message.toolCalls && message.toolCalls.length > 0 && (
          <div className="mt-2 space-y-1">
            {message.toolCalls.map((tc, i) => (
              <div key={i} className="text-xs px-2 py-1 rounded"
                   style={{ background: 'var(--bg-primary)', color: 'var(--text-secondary)' }}>
                <span style={{ color: 'var(--accent)' }}>{tc.tool}</span>
                {tc.result && <span className="ml-1" style={{ color: 'var(--success)' }}>done</span>}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
