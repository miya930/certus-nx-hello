interface Props<T extends string> {
  tabs: readonly T[];
  active: T;
  onChange: (tab: T) => void;
}

export function Tabs<T extends string>({ tabs, active, onChange }: Props<T>) {
  return (
    <div className="tabs" role="tablist">
      {tabs.map((tab) => (
        <button
          key={tab}
          type="button"
          role="tab"
          aria-selected={tab === active}
          className={tab === active ? "active" : ""}
          onClick={() => onChange(tab)}
        >
          {tab}
        </button>
      ))}
    </div>
  );
}
