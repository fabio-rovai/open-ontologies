import { useState, useRef, useEffect } from 'react';

interface AddClassDialogProps {
  position: { x: number; y: number };
  onSubmit: (className: string) => void;
  onCancel: () => void;
}

export function AddClassDialog({ position, onSubmit, onCancel }: AddClassDialogProps) {
  const [value, setValue] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter' && value.trim()) {
      e.preventDefault();
      onSubmit(value.trim());
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
  }

  return (
    <div
      className="absolute z-50 flex flex-col gap-1"
      style={{
        left: position.x,
        top: position.y,
      }}
    >
      <input
        ref={inputRef}
        type="text"
        placeholder="Class name..."
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={onCancel}
        className="px-3 py-1.5 rounded text-sm outline-none"
        style={{
          background: 'var(--bg-panel)',
          color: 'var(--text-primary)',
          border: '1px solid var(--accent)',
          minWidth: '180px',
        }}
      />
      <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>
        Enter to create &middot; Esc to cancel
      </span>
    </div>
  );
}
