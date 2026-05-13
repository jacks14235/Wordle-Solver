import { useMemo, useRef, useState } from 'react'
import './App.css'
import type { SolverProgress, TrialGame } from './solverWorker'

type WorkerMessage =
  | { type: 'ready'; wordCount: number }
  | { type: 'trials'; trials: TrialGame[] }
  | { type: 'progress'; progress: SolverProgress }
  | { type: 'done'; possibleWords: string[] }
  | { type: 'error'; message: string }

type RunStatus = 'idle' | 'running' | 'done' | 'error'
type ActiveTab = 'solver' | 'how-it-works'

const emptyProgress = {
  prefilter: undefined,
  hardMode: undefined,
} satisfies {
  prefilter?: SolverProgress
  hardMode?: SolverProgress
}

function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>('solver')
  const [answer, setAnswer] = useState('fjord')
  const [nTrials, setNTrials] = useState(10)
  const [seed, setSeed] = useState('1')
  const [hardMode, setHardMode] = useState(true)
  const [showLetters, setShowLetters] = useState(false)
  const [status, setStatus] = useState<RunStatus>('idle')
  const [wordCount, setWordCount] = useState<number | null>(null)
  const [trials, setTrials] = useState<TrialGame[]>([])
  const [possibleWords, setPossibleWords] = useState<string[]>([])
  const [progress, setProgress] = useState(emptyProgress)
  const [errorMessage, setErrorMessage] = useState('')
  const workerRef = useRef<Worker | null>(null)

  const flattenedRows = useMemo(
    () => trials.reduce((count, trial) => count + trial.guesses.length, 0),
    [trials],
  )

  const runSolver = () => {
    workerRef.current?.terminate()

    const worker = new Worker(new URL('./solverWorker.ts', import.meta.url), {
      type: 'module',
    })
    workerRef.current = worker

    setStatus('running')
    setTrials([])
    setPossibleWords([])
    setProgress(emptyProgress)
    setErrorMessage('')

    worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
      const message = event.data

      if (message.type === 'ready') {
        setWordCount(message.wordCount)
        return
      }

      if (message.type === 'trials') {
        setTrials(message.trials)
        return
      }

      if (message.type === 'progress') {
        setProgress((current) => ({
          ...current,
          [message.progress.stage === 'prefilter' ? 'prefilter' : 'hardMode']:
            message.progress,
        }))
        return
      }

      if (message.type === 'done') {
        setPossibleWords(message.possibleWords)
        setStatus('done')
        worker.terminate()
        workerRef.current = null
        return
      }

      setErrorMessage(message.message)
      setStatus('error')
      worker.terminate()
      workerRef.current = null
    }

    worker.postMessage({
      type: 'solve',
      answer: answer.trim().toLowerCase(),
      nTrials,
      seed,
      hardMode,
    })
  }

  const cancelSolver = () => {
    workerRef.current?.terminate()
    workerRef.current = null
    setStatus('idle')
  }

  return (
    <main className="app-shell">
      <section className="hero-panel">
        <div>
          <p className="eyebrow">Hidden Wordle solver</p>
          <h1>Generate hard-mode score grids and solve them in WASM.</h1>
          <p className="lede">
            The Rust solver runs in a Web Worker so the grids and progress bars
            keep updating while the candidate search runs.
          </p>
        </div>

        <form
          className="controls"
          onSubmit={(event) => {
            event.preventDefault()
            runSolver()
          }}
        >
          <label>
            Answer
            <input
              value={answer}
              maxLength={5}
              onChange={(event) => setAnswer(event.target.value)}
            />
          </label>

          <label>
            Games
            <input
              type="number"
              min={1}
              max={50}
              value={nTrials}
              onChange={(event) => setNTrials(Number(event.target.value))}
            />
          </label>

          <label>
            Seed
            <input
              value={seed}
              onChange={(event) => setSeed(event.target.value)}
            />
          </label>

          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={hardMode}
              onChange={(event) => setHardMode(event.target.checked)}
            />
            Hard-mode trial generation
          </label>

          <div className="button-row">
            <button disabled={status === 'running'} type="submit">
              {status === 'running' ? 'Solving...' : 'Generate and solve'}
            </button>
            <button
              disabled={status !== 'running'}
              type="button"
              className="secondary"
              onClick={cancelSolver}
            >
              Cancel
            </button>
          </div>
        </form>
      </section>

      <nav className="tab-list" aria-label="Page sections">
        <button
          className={activeTab === 'solver' ? 'tab-button active' : 'tab-button'}
          type="button"
          onClick={() => setActiveTab('solver')}
        >
          Solver
        </button>
        <button
          className={activeTab === 'how-it-works' ? 'tab-button active' : 'tab-button'}
          type="button"
          onClick={() => setActiveTab('how-it-works')}
        >
          How it works
        </button>
      </nav>

      {activeTab === 'solver' ? (
        <>
          <section className="stats-grid">
            <Stat label="Dictionary" value={wordCount?.toLocaleString() ?? '...'} />
            <Stat label="Games" value={trials.length.toLocaleString()} />
            <Stat label="Rows" value={flattenedRows.toLocaleString()} />
            <Stat label="Possible answers" value={possibleWords.length.toLocaleString()} />
          </section>

          <section className="progress-panel">
            <h2>Solver Progress</h2>
            <ProgressBar
              fallbackTotal={flattenedRows}
              progress={progress.prefilter}
              title="Independent row prefilter"
            />
            <ProgressBar
              progress={progress.hardMode}
              title="Sequential hard-mode validation"
            />
            {status === 'error' && <p className="error">{errorMessage}</p>}
          </section>

          <section className="content-grid">
            <div className="panel">
              <div className="panel-heading">
                <h2>Generated Games</h2>
                <label className="checkbox-row compact">
                  <input
                    type="checkbox"
                    checked={showLetters}
                    onChange={(event) => setShowLetters(event.target.checked)}
                  />
                  Show guess letters
                </label>
              </div>
              <div className="trial-list">
                {trials.map((trial, trialIdx) => (
                  <TrialCard
                    key={`${trial.answer}-${trialIdx}`}
                    trial={trial}
                    index={trialIdx}
                    showLetters={showLetters}
                  />
                ))}
              </div>
            </div>

            <div className="panel">
              <h2>Surviving Answers</h2>
              <div className="word-list">
                {possibleWords.map((word) => (
                  <span key={word}>{word}</span>
                ))}
              </div>
            </div>
          </section>
        </>
      ) : (
        <HowItWorks />
      )}
    </main>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat-card">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  )
}

function ProgressBar({
  fallbackTotal,
  progress,
  title,
}: {
  fallbackTotal?: number
  progress?: SolverProgress
  title: string
}) {
  const total = progress?.total ?? fallbackTotal ?? 0
  const current = progress?.current ?? 0
  const percent = total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0

  return (
    <div className="progress-row">
      <div className="progress-heading">
        <span>{title}</span>
        <span>{percent}%</span>
      </div>
      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${percent}%` }} />
      </div>
      <p>
        {progress?.label ?? 'Waiting to start'} · remaining{' '}
        {(progress?.remaining ?? 0).toLocaleString()}
      </p>
    </div>
  )
}

function TrialCard({
  index,
  showLetters,
  trial,
}: {
  index: number
  showLetters: boolean
  trial: TrialGame
}) {
  return (
    <article className="trial-card">
      <div className="trial-heading">
        <h3>Trial {index + 1}</h3>
        <span>{trial.guesses.at(-1)?.remaining ?? 0} remaining</span>
      </div>
      <div className="wordle-grid">
        {trial.guesses.map((guess, guessIdx) => (
          <div className="wordle-row" key={`${guess.word}-${guessIdx}`}>
            {guess.word.split('').map((letter, letterIdx) => (
              <span
                aria-label={showLetters ? letter : 'hidden letter'}
                className={`tile tile-${guess.match[letterIdx]?.toLowerCase() ?? 'b'} ${
                  showLetters ? '' : 'tile-hidden'
                }`}
                key={`${letter}-${letterIdx}`}
              >
                {showLetters ? letter : ''}
              </span>
            ))}
            <small>{guess.remaining}</small>
          </div>
        ))}
      </div>
    </article>
  )
}

function HowItWorks() {
  return (
    <section className="how-panel">
      <div className="how-intro">
        <p className="eyebrow">Background</p>
        <h2>What can a hidden Wordle grid reveal?</h2>
        <p>
          Wordle share grids hide the actual guesses, but each row still says how one unknown
          five-letter word scored against the true answer. This app asks which answers are still
          compatible with those visible score patterns.
        </p>
      </div>

      <div className="how-grid">
        <article className="how-card">
          <span className="step-number">1</span>
          <h3>Generate hidden games</h3>
          <p>
            The generator picks a known answer, simulates random players, and keeps only their
            colored result rows. In hard mode, each simulated next guess must reuse the green and
            yellow information revealed so far.
          </p>
        </article>

        <article className="how-card">
          <span className="step-number">2</span>
          <h3>Run a fast prefilter</h3>
          <p>
            The first pass treats every row independently. For each possible answer, it checks
            whether some hidden guess could have produced each observed pattern. Answers that fail
            this broad test are impossible.
          </p>
        </article>

        <article className="how-card">
          <span className="step-number">3</span>
          <h3>Validate hard-mode paths</h3>
          <p>
            The second pass preserves the order of each player's rows. It searches for a sequence
            of hidden guesses that both produces the visible grid and obeys the accumulated
            hard-mode constraints.
          </p>
        </article>
      </div>

      <div className="panel explanation-panel">
        <h2>Why answers remain</h2>
        <p>
          Even with hard mode, the actual guess words are unknown. Many different guesses can lead
          to the same visible row, so the solver can rule out contradictions but cannot uniquely
          identify the answer unless the grids are restrictive enough.
        </p>
        <p>
          More games usually narrow the answer set further. Stronger assumptions, like common
          starting words or players choosing likely answers, would add more signal but would also
          model human behavior rather than pure Wordle legality.
        </p>
      </div>
    </section>
  )
}

export default App
