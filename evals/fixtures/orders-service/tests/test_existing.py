"""The behaviour that already works. Must keep working."""
from store.cart import Cart
from store import catalog, orders
from billing import tax


def run():
    c = Cart()
    c.add("sku-1", 2)
    assert c.subtotal_cents() == 5000

    assert tax.rate_for("US-OR") == 0.0
    assert tax.tax_cents(5000, "US-CA") == 463
    assert tax.tax_cents(5000, "US-OR") == 0

    inv = orders.place(c, "US-OR")
    assert inv["subtotal_cents"] == 5000
    assert inv["total_cents"] == 5000
    assert catalog.get("sku-1")["stock"] == 8

    empty = Cart()
    try:
        orders.place(empty, "GB")
    except ValueError:
        pass
    else:
        raise AssertionError("empty order should raise")
    print("EXISTING_OK")


run()
