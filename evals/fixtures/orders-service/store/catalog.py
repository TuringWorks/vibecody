"""Product catalogue. Prices are integer cents throughout."""

_PRODUCTS = {
    "sku-1": {"name": "Widget", "price_cents": 2500, "stock": 10},
    "sku-2": {"name": "Gadget", "price_cents": 17999, "stock": 3},
    "sku-3": {"name": "Doohickey", "price_cents": 499, "stock": 0},
}


def get(sku):
    """Return the product, or None when the sku is unknown."""
    product = _PRODUCTS.get(sku)
    return dict(product) if product else None


def in_stock(sku, qty):
    product = _PRODUCTS.get(sku)
    return product is not None and product["stock"] >= qty


def reserve(sku, qty):
    """Take `qty` out of stock. Returns False when there is not enough."""
    if not in_stock(sku, qty):
        return False
    _PRODUCTS[sku]["stock"] -= qty
    return True


def all_skus():
    return sorted(_PRODUCTS)
