import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import { localStorageKey } from '@/constants/localStorage'

type ChatModesState = {
  draftIncognitoEnabled: boolean
  braveGroundingThreads: Record<string, boolean>
  toggleDraftIncognito: () => void
  setDraftIncognitoEnabled: (enabled: boolean) => void
  isBraveGroundingEnabled: (threadId: string) => boolean
  toggleBraveGrounding: (threadId: string) => void
  setBraveGrounding: (threadId: string, enabled: boolean) => void
  transferBraveGrounding: (fromThreadId: string, toThreadId: string) => void
  removeThread: (threadId: string) => void
}

export const useChatModes = create<ChatModesState>()(
  persist(
    (set, get) => ({
      draftIncognitoEnabled: false,
      braveGroundingThreads: {},

      toggleDraftIncognito: () => {
        set((state) => ({
          draftIncognitoEnabled: !state.draftIncognitoEnabled,
        }))
      },

      setDraftIncognitoEnabled: (enabled) => {
        set({ draftIncognitoEnabled: enabled })
      },

      isBraveGroundingEnabled: (threadId) => {
        return get().braveGroundingThreads[threadId] === true
      },

      toggleBraveGrounding: (threadId) => {
        set((state) => ({
          braveGroundingThreads: {
            ...state.braveGroundingThreads,
            [threadId]: !state.braveGroundingThreads[threadId],
          },
        }))
      },

      setBraveGrounding: (threadId, enabled) => {
        set((state) => ({
          braveGroundingThreads: {
            ...state.braveGroundingThreads,
            [threadId]: enabled,
          },
        }))
      },

      transferBraveGrounding: (fromThreadId, toThreadId) => {
        const enabled = get().braveGroundingThreads[fromThreadId] === true
        set((state) => {
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
          const { [fromThreadId]: _removed, ...rest } =
            state.braveGroundingThreads

          return {
            braveGroundingThreads: enabled
              ? {
                  ...rest,
                  [toThreadId]: true,
                }
              : rest,
          }
        })
      },

      removeThread: (threadId) => {
        set((state) => {
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
          const { [threadId]: _removed, ...rest } = state.braveGroundingThreads
          return { braveGroundingThreads: rest }
        })
      },
    }),
    {
      name: localStorageKey.chatModes,
      storage: createJSONStorage(() => localStorage),
    }
  )
)
