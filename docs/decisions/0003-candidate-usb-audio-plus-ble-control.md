# 0003 — Candidate USB audio plus BLE control split

- Status: Proposed
- Date: 2026-07-29

## Context

Bidirectional audio benefits from a standard low-latency host audio path. Radio CAT and
PTT require little bandwidth but must be available to a distributable iOS application.
Generic desktop USB-serial assumptions may not apply to iPhone.

## Candidate

Use a class-compliant USB audio device for receive/transmit audio and a versioned
Bluetooth Low Energy service for discovery, CAT/control, PTT leases, status, and
configuration.

## Why this is not accepted

M0 must verify current Apple platform APIs, entitlement and distribution constraints,
background behavior, USB composite alternatives, BLE latency/reliability, and the
single-tether product implications. The transport decision issue may accept, revise, or
reject this candidate.
