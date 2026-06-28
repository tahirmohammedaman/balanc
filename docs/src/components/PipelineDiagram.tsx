export function PipelineDiagram() {
  return (
    <div className="pipeline-wrap">
      <svg
        className="pipeline-svg"
        viewBox="0 0 980 190"
        role="img"
        aria-label="Compiler pipeline: source text through lex, parse, resolve, typeck, then either eval or the JVM backend"
      >
        <g fontSize={12.5}>
          <g>
            <rect className="stage-box" x="10" y="55" width="96" height="46" rx="8" />
            <text className="lbl" x="58" y="83" textAnchor="middle">
              source
            </text>
          </g>
          <g>
            <rect className="stage-box" x="146" y="55" width="96" height="46" rx="8" />
            <text className="lbl" x="194" y="83" textAnchor="middle">
              lex
            </text>
          </g>
          <g>
            <rect className="stage-box" x="282" y="55" width="96" height="46" rx="8" />
            <text className="lbl" x="330" y="83" textAnchor="middle">
              parse
            </text>
          </g>
          <g>
            <rect className="stage-box" x="418" y="55" width="96" height="46" rx="8" />
            <text className="lbl" x="466" y="83" textAnchor="middle">
              resolve
            </text>
          </g>
          <g>
            <rect className="stage-box hl" x="554" y="55" width="110" height="46" rx="8" />
            <text className="lbl" x="609" y="83" textAnchor="middle">
              typeck
            </text>
          </g>
          <g>
            <rect className="stage-box" x="722" y="10" width="120" height="46" rx="8" />
            <text className="lbl" x="782" y="38" textAnchor="middle">
              eval
            </text>
          </g>
          <g>
            <rect className="branch-box" x="722" y="100" width="120" height="46" rx="8" />
            <text className="lbl" x="782" y="128" textAnchor="middle">
              backend::jvm
            </text>
          </g>
          <g>
            <rect className="stage-box" x="880" y="10" width="90" height="46" rx="8" />
            <text className="lbl" x="925" y="38" textAnchor="middle" fontSize={11.5}>
              report
            </text>
          </g>
          <g>
            <rect className="stage-box" x="880" y="100" width="90" height="46" rx="8" />
            <text className="lbl" x="925" y="128" textAnchor="middle" fontSize={11.5}>
              .class
            </text>
          </g>
          <g className="arrow" strokeWidth={1.5} markerEnd="url(#pipeline-arrowhead)">
            <line x1="106" y1="78" x2="144" y2="78" />
            <line x1="242" y1="78" x2="280" y2="78" />
            <line x1="378" y1="78" x2="416" y2="78" />
            <line x1="514" y1="78" x2="552" y2="78" />
            <line x1="664" y1="66" x2="720" y2="40" />
            <line x1="664" y1="90" x2="720" y2="118" />
            <line x1="842" y1="33" x2="878" y2="33" />
            <line x1="842" y1="123" x2="878" y2="123" />
          </g>
        </g>
        <defs>
          <marker id="pipeline-arrowhead" markerWidth={8} markerHeight={8} refX={6} refY={3} orient="auto">
            <path d="M0,0 L6,3 L0,6 Z" fill="var(--text-faint)" />
          </marker>
        </defs>
        <text className="sub" x="609" y="163" textAnchor="middle">
          TModule — the shared typed IR
        </text>
      </svg>
    </div>
  );
}
