/**
 * What the TLS inspector says about a certificate.
 *
 * The panel used to render one boolean — "Valid" or "Invalid / Expired" — and
 * a bare number of days. A backend that could not read the dates therefore came
 * out as a site whose certificate had expired today, which is what a parsing
 * bug looked like for every site on the internet. These pin the four outcomes
 * apart on screen.
 */
import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import { NetworkPanel } from '../NetworkPanel';

const CERT = {
  subject: 'CN=github.com',
  issuer: 'C=GB, O=Sectigo Limited, CN=Sectigo Public Server Authentication CA DV E36',
  not_before: 'Sep  1 00:00:00 2026 GMT',
  not_after: 'Nov 29 23:59:59 2026 GMT',
  san: ['github.com', 'www.github.com'],
  serial: 'A59EBDB596751DB7F5C095079613953C',
  valid: true,
  status: 'Valid',
  days_remaining: 88,
  raw: 'Verify return code: 0 (ok)',
};

/** Open the TLS tab, name a host, and let the mocked command resolve. */
async function inspect(cert: Record<string, unknown>) {
  invoke.mockResolvedValue(cert);
  render(<NetworkPanel />);
  fireEvent.click(screen.getByRole('button', { name: /tls/i }));
  fireEvent.change(screen.getByPlaceholderText(/example\.com/i), {
    target: { value: 'github.com' },
  });
  fireEvent.click(screen.getByRole('button', { name: /^check$/i }));
}

beforeEach(() => invoke.mockReset());

describe('the TLS inspector', () => {
  it('shows a working certificate as valid, with the days it has left', async () => {
    await inspect(CERT);

    expect(await screen.findByText('Valid')).toBeInTheDocument();
    expect(screen.getByText('88d')).toBeInTheDocument();
    expect(screen.getByText('remaining')).toBeInTheDocument();
    expect(screen.getByText('CN=github.com')).toBeInTheDocument();
    expect(screen.getByText('Nov 29 23:59:59 2026 GMT')).toBeInTheDocument();
  });

  it('says how long ago an expired one expired, rather than counting down from zero', async () => {
    await inspect({
      ...CERT,
      valid: false,
      status: 'Chain not trusted — certificate has expired',
      days_remaining: -4160,
      not_after: 'Apr 12 23:59:59 2015 GMT',
    });

    expect(await screen.findByText(/certificate has expired/)).toBeInTheDocument();
    expect(screen.getByText('4160d')).toBeInTheDocument();
    expect(screen.getByText('since it expired')).toBeInTheDocument();
  });

  it('names an untrusted chain as untrusted, and still shows its dates', async () => {
    await inspect({
      ...CERT,
      valid: false,
      status: 'Chain not trusted — self-signed certificate',
      days_remaining: 729,
    });

    expect(await screen.findByText(/self-signed certificate/)).toBeInTheDocument();
    // The certificate is readable; only the chain is not. Its expiry is a fact.
    expect(screen.getByText('729d')).toBeInTheDocument();
  });

  it('does not report an unreadable certificate as an expired one', async () => {
    await inspect({
      ...CERT,
      valid: false,
      status: 'Could not read the certificate',
      not_after: '',
      days_remaining: null,
    });

    expect(await screen.findByText('Could not read the certificate')).toBeInTheDocument();
    // No number: "0d remaining" is a claim about a date nobody could read.
    expect(screen.queryByText('0d')).not.toBeInTheDocument();
    expect(screen.getByText('expiry unknown')).toBeInTheDocument();
  });
});
