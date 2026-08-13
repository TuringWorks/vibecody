"""In-memory carts."""
from store import catalog


class Cart:
    def __init__(self):
        self.lines = {}

    def add(self, sku, qty):
        if qty < 1:
            raise ValueError("qty must be positive")
        if catalog.get(sku) is None:
            raise KeyError(sku)
        self.lines[sku] = self.lines.get(sku, 0) + qty

    def subtotal_cents(self):
        total = 0
        for sku, qty in self.lines.items():
            total += catalog.get(sku)["price_cents"] * qty
        return total

    def is_empty(self):
        return not self.lines
