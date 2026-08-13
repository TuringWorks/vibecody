"""Order placement — the seam between cart, catalogue and billing."""
from billing import invoice


def place(cart, region):
    """Reserve stock and produce an invoice.

    Raises ValueError on an empty cart.
    """
    if cart.is_empty():
        raise ValueError("cannot place an empty order")
    from store import catalog
    for sku, qty in cart.lines.items():
        if not catalog.reserve(sku, qty):
            raise ValueError(f"insufficient stock for {sku}")
    return invoice.build(cart, region)
