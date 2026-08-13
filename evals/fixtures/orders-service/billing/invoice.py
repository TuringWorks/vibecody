"""Invoice assembly."""
from billing import tax


def build(cart, region):
    subtotal = cart.subtotal_cents()
    t = tax.tax_cents(subtotal, region)
    return {
        "subtotal_cents": subtotal,
        "tax_cents": t,
        "total_cents": subtotal + t,
        "region": region,
    }
