export default function App() {
  return (
    <div className="flex h-full items-center justify-center p-3">
      <div
        className="flex h-full w-full items-center justify-center border"
        style={{
          background: "var(--cs-surface)",
          borderColor: "var(--cs-border)",
          borderRadius: "var(--cs-radius-card)",
        }}
      >
        <span style={{ color: "var(--cs-text-muted)" }}>Clipstack</span>
      </div>
    </div>
  );
}
