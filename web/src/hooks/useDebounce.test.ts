import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useDebounce } from './useDebounce'

describe('useDebounce', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('returns the initial value immediately', () => {
    const { result } = renderHook(() => useDebounce('jazz', 300))
    expect(result.current).toBe('jazz')
  })

  it('only exposes the new value after the delay', () => {
    const { result, rerender } = renderHook(({ value }) => useDebounce(value, 300), {
      initialProps: { value: 'jazz' },
    })

    rerender({ value: 'jazz fm' })
    expect(result.current).toBe('jazz')

    act(() => vi.advanceTimersByTime(299))
    expect(result.current).toBe('jazz')

    act(() => vi.advanceTimersByTime(1))
    expect(result.current).toBe('jazz fm')
  })

  it('resets the timer on rapid changes', () => {
    const { result, rerender } = renderHook(({ value }) => useDebounce(value, 300), {
      initialProps: { value: 'j' },
    })

    rerender({ value: 'ja' })
    act(() => vi.advanceTimersByTime(200))
    rerender({ value: 'jaz' })
    act(() => vi.advanceTimersByTime(200))
    expect(result.current).toBe('j')

    act(() => vi.advanceTimersByTime(300))
    expect(result.current).toBe('jaz')
  })
})
