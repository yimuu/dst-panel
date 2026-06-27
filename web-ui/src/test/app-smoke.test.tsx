import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import App from '@/app/App'

describe('App', () => {
  it('renders the scaffold title', () => {
    render(<App />)

    expect(screen.getByText('饥荒联机版管理面板')).toBeInTheDocument()
  })
})
