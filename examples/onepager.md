# Helios Reconciliation

A one-pager for a new internal tool that reconciles vendor invoices against
purchase orders, so finance stops doing it by hand.

## Problem

Finance analysts spend hours every week matching invoices to POs in
spreadsheets. It is slow, error-prone, and nobody trusts the numbers at
quarter-end.

## Approach

A small service ingests invoices and POs, matches them on amount and vendor,
and flags the exceptions a human must look at. Deterministic first, with a
review queue for the ambiguous cases.

### Ingest

Pull invoices and purchase orders from the existing accounting export. No new
data entry; read what is already there.

### Match

Pair each invoice with its purchase order on vendor and amount, within a
tolerance. Confident matches clear automatically.

### Review queue

Everything the matcher is unsure about lands in a queue for an analyst to
resolve, so no exception is silently dropped.

## Risks

Vendor names are inconsistent across systems, and partial shipments make
amounts disagree. Both will need careful normalization.
