import {
  DRAFT_CHAT_MODE_ID,
  TEMPORARY_CHAT_ID,
} from '@/constants/chat'
import { useAgentMode } from '@/hooks/useAgentMode'
import { useChatAttachments } from '@/hooks/useChatAttachments'
import { useChatModes } from '@/hooks/useChatModes'
import { useMessages } from '@/hooks/useMessages'
import { useThreads } from '@/hooks/useThreads'
import { useChatSessions } from '@/stores/chat-session-store'

export function clearDraftChatModes() {
  useChatModes.getState().setDraftIncognitoEnabled(false)
  useChatModes.getState().removeThread(DRAFT_CHAT_MODE_ID)
}

export function resetTemporaryChatState() {
  useChatSessions.getState().removeSession(TEMPORARY_CHAT_ID)
  useMessages.getState().setMessages(TEMPORARY_CHAT_ID, [])
  useChatAttachments.getState().clearAttachments(TEMPORARY_CHAT_ID)
  useThreads.getState().deleteThread(TEMPORARY_CHAT_ID)
  useAgentMode.getState().removeThread(TEMPORARY_CHAT_ID)
  useChatModes.getState().removeThread(TEMPORARY_CHAT_ID)
}
