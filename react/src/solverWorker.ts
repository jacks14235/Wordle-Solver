import init, {
  generate_trials,
  hard_mode_match_exists_with_progress,
  word_count,
} from './wasm/wordle_stuff.js'

export type TrialGuess = {
  word?: string
  match: string
  remaining?: number
}

export type TrialGame = {
  answer?: string
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
  mode: 'generate' | 'paste'
  answer: string
  nTrials: number
  pastedGrids: string
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

function parsePastedGames(input: string): TrialGame[] {
  const games: TrialGame[] = []
  let guesses: TrialGuess[] = []

  for (const rawLine of input.split(/\r?\n/)) {
    const line = rawLine.replace(/[^\S\n]/gu, '')
    if (line.length === 0) {
      continue
    }

    if (/^=+$/u.test(line)) {
      if (guesses.length > 0) {
        games.push({ guesses })
        guesses = []
      }
      continue
    }

    if (Array.from(line).length !== 5) {
      throw new Error(`Expected five tiles in pasted row: ${rawLine}`)
    }

    guesses.push({ match: line })
  }

  if (guesses.length > 0) {
    games.push({ guesses })
  }

  if (games.length === 0) {
    throw new Error('Paste at least one five-tile Wordle row.')
  }

  return games
}

self.onmessage = async (event: MessageEvent<SolveMessage>) => {
  if (event.data.type !== 'solve') {
    return
  }

  try {
    await wasmReady

    const trials =
      event.data.mode === 'paste'
        ? parsePastedGames(event.data.pastedGrids)
        : (generate_trials(
            event.data.answer,
            event.data.nTrials,
            BigInt(event.data.seed || Date.now()),
            event.data.hardMode,
          ) as TrialGame[])

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
