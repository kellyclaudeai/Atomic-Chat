/**
 * Chat-related constants
 */

export const TEMPORARY_CHAT_ID = 'temporary-chat'
export const TEMPORARY_CHAT_QUERY_ID = 'temporary-chat'
export const DRAFT_CHAT_MODE_ID = '__draft-chat__'

/**
 * Session storage keys for initial messages
 */
export const SESSION_STORAGE_KEY = {
  INITIAL_MESSAGE_TEMPORARY: 'initial-message-temporary',
} as const

export const SESSION_STORAGE_PREFIX = {
  INITIAL_MESSAGE: 'initial-message-',
} as const
