"""Tax rules, kept separate from pricing on purpose."""

RATES = {"US-CA": 0.0925, "US-OR": 0.0, "GB": 0.20}


def rate_for(region):
    if region not in RATES:
        raise KeyError(f"no tax rate for region {region}")
    return RATES[region]


def tax_cents(subtotal_cents, region):
    """Tax owed, rounded half-up to the nearest cent."""
    rate = rate_for(region)
    return int(subtotal_cents * rate + 0.5)
