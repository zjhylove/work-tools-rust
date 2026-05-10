import { ViewMode } from '../types'

interface Props {
  view: ViewMode
}

const steps = [
  { key: 'config', label: '配置', num: 1 },
  { key: 'select', label: '选择 & 解析', num: 2 },
  { key: 'preview', label: '导出', num: 3 },
] as const

function stepState(key: string, view: ViewMode): 'active' | 'done' | 'pending' {
  const order = ['config', 'select', 'preview']
  const idx = order.indexOf(key)
  const cur = order.indexOf(view)
  if (idx === cur) return 'active'
  if (idx < cur) return 'done'
  return 'pending'
}

export default function StepHeader({ view }: Props) {
  return (
    <header className="step-header">
      <div className="step-header-track">
        {steps.map((step, i) => {
          const state = stepState(step.key, view)
          return (
            <div key={step.key} className="step-header-group">
              <div className={`step-node step-node--${state}`}>
                {state === 'done' ? (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                ) : (
                  <span className="step-node-num">{step.num}</span>
                )}
              </div>
              <span className={`step-label step-label--${state}`}>{step.label}</span>
              {i < steps.length - 1 && (
                <div className={`step-connector step-connector--${stepState(steps[i + 1].key, view) === 'pending' ? 'pending' : 'filled'}`} />
              )}
            </div>
          )
        })}
      </div>
    </header>
  )
}
