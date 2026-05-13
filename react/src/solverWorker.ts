import init, {
  generate_trials,
  hard_mode_match_exists_with_progress,
  word_count,
} from '../../rust_version/pkg/wordle_stuff.js'

export type TrialGuess = {
  word: string
  match: string
  remaining: number
}

export type TrialGame = {
  answer: string
  guesses: TrialGuess[]
}

export type SolverProgress = {
  stage: 'prefilter' | 'hard-mode'
  current: number
  total: number
  remaining: number
  label: string
}

type SolveMessage = {
  type: 'solve'
  answer: string
  nTrials: number
  seed: string
  hardMode: boolean
}

type WorkerMessage =
  | { type: 'ready'; wordCount: number }
  | { type: 'trials'; trials: TrialGame[] }
  | { type: 'progress'; progress: SolverProgress }
  | { type: 'done'; possibleWords: string[] }
  | { type: 'error'; message: string }

const send = (message: WorkerMessage) => self.postMessage(message)

const wasmReady = init().then(() => {
  send({ type: 'ready', wordCount: word_count() })
})

self.onmessage = async (event: MessageEvent<SolveMessage>) => {
  if (event.data.type !== 'solve') {
    return
  }

  try {
    await wasmReady

    const seed = BigInt(event.data.seed || Date.now())
    const trials = generate_trials(
      event.data.answer,
      event.data.nTrials,
      seed,
      event.data.hardMode,
    ) as TrialGame[]

    send({ type: 'trials', trials })

    const possibleWords = hard_mode_match_exists_with_progress(
      trials,
      (progress: SolverProgress) => send({ type: 'progress', progress }),
    ) as string[]

    send({ type: 'done', possibleWords })
  } catch (error) {
    send({
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    })
  }
}
