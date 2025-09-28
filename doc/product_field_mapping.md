# Product Field Mapping Documentation

## CSV Column to Entity Field Mapping

| CSV Column (German) | Entity Field (English) | Type | Description | Example Values |
|-------------------|---------------------|------|-------------|----------------|
| EAN | ean | String | Product barcode/identifier | "13352", "426247218083" |
| Bezeichnung | name | String | Full product name | "Ochsenherztomaten I", "Macadamia süss salzig" |
| Kurzbezeichnung | short_name | String | Abbreviated product name | "Ochsenherztomate", "Macadamia süss s" |
| VKEinheit | sales_unit | String | Unit of sale | "kg", "St", "l", "100g", "0,33l" |
| WiegeArtikel | requires_weighing | bool | Weighing required flag | 9 → true, 0 → false |
| VKHerst | price | Price | Sales price (stored as cents) | "5,39" → 539 cents |

## Product Entity Structure

```rust
#[derive(Debug, Clone)]
pub struct Product {
    // Technical identifier
    pub id: Uuid,                       // Technical UUID identifier
    
    // Core product fields
    pub ean: String,                    // EAN barcode (unique business identifier)
    pub name: String,                   // Full product name
    pub short_name: String,             // Short name (for display)
    pub sales_unit: String,             // Sales unit: "kg", "St", "l", "100g", "0,33l"
    pub requires_weighing: bool,        // true = needs weighing, false = count pieces
    pub price: Price,                   // Sales price in cents (custom type)
    
    // Standard inventurly fields
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}
```

## Custom Price Type

The `Price` type stores monetary values as cents (integer) to avoid floating-point precision issues:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price {
    cents: i64,  // Store price in cents to avoid floating point issues
}

impl Price {
    pub fn from_cents(cents: i64) -> Self {
        Self { cents }
    }
    
    pub fn from_euros(euros: f64) -> Self {
        Self {
            cents: (euros * 100.0).round() as i64,
        }
    }
    
    pub fn to_cents(&self) -> i64 {
        self.cents
    }
    
    pub fn to_euros(&self) -> f64 {
        self.cents as f64 / 100.0
    }
}
```

- **Internal storage**: `i64` cents (539 = €5.39)
- **CSV parsing**: Handles German format with comma decimal separator
- **Database storage**: INTEGER field storing cents
- **Display**: Can convert to euros for display (539 → 5.39)

### Price Conversion Examples

| CSV Value | Parsed Euros | Stored Cents | Display |
|-----------|--------------|--------------|---------|
| "5,39" | 5.39 | 539 | €5.39 |
| "1,79" | 1.79 | 179 | €1.79 |
| "0,00" | 0.00 | 0 | €0.00 |
| "12,95" | 12.95 | 1295 | €12.95 |

## Translation Notes

- **Bezeichnung** → **name**: "Designation/Description" translated to simpler "name"
- **Kurzbezeichnung** → **short_name**: "Short designation" for mobile/compact displays
- **VKEinheit** → **sales_unit**: "Verkaufseinheit" (sales unit) - the unit customers purchase
- **WiegeArtikel** → **requires_weighing**: "Weighing article" - indicates if item needs to be weighed
- **VKHerst** → **price**: "Verkauf Hersteller" (manufacturer sales price) stored as cents

## Unit Translations

| German | English | Type |
|--------|---------|------|
| St | pcs | Pieces (Stück) |
| kg | kg | Kilograms |
| l | l | Liters |
| g | g | Grams |

## Data Conversions

- **Decimal separator**: German comma (,) → period (.) → cents integer
- **WiegeArtikel values**: 9 = requires weighing (true), 0 = count pieces (false)
- **Empty prices**: "0,00" or empty → 0 cents
- **Price parsing**: "5,39" → 5.39 euros → 539 cents

## Example Data Mapping

### CSV Input
```csv
"426247218083","Macadamia süss salzig","Macadamia süss s","130g","D","0303","C%","0","TAA","","","0","5,39",...
```

### Mapped to Product Entity
```rust
Product {
    id: Uuid::new_v4(),
    ean: "426247218083".to_string(),
    name: "Macadamia süss salzig".to_string(),
    short_name: "Macadamia süss s".to_string(),
    sales_unit: "130g".to_string(),
    requires_weighing: false,  // 0 → false
    price: Price::from_cents(539),  // "5,39" → 539 cents
    created: time::PrimitiveDateTime::now(),
    deleted: None,
    version: Uuid::new_v4(),
}
```

## Benefits of Cents Storage

1. **No floating-point errors**: Integer arithmetic is precise
2. **Easy calculations**: Sum prices without rounding issues  
3. **Database efficiency**: INTEGER is faster than DECIMAL
4. **Standard practice**: Many payment systems use cents internally
5. **Cross-language compatibility**: Integers work the same everywhere

## Database Schema

The product table should have the following structure:

```sql
CREATE TABLE product (
    id BLOB PRIMARY KEY,           -- UUID as bytes
    ean TEXT NOT NULL UNIQUE,      -- Barcode, unique index for lookups
    name TEXT NOT NULL,
    short_name TEXT NOT NULL,
    sales_unit TEXT NOT NULL,
    requires_weighing INTEGER NOT NULL,  -- 0 or 1
    price INTEGER NOT NULL,         -- Price in cents
    created TEXT NOT NULL,          -- ISO8601 timestamp
    deleted TEXT,                   -- ISO8601 timestamp or NULL
    version BLOB NOT NULL           -- UUID as bytes
);

CREATE UNIQUE INDEX idx_product_ean ON product(ean);
```

## Import Considerations

When importing from CSV:
1. Parse the CSV file line by line
2. Skip the header row
3. Convert German decimal format (comma) to standard format (period)
4. Convert "WiegeArtikel" values: 9 → true, 0 → false
5. Parse price string to euros, then convert to cents
6. Generate UUIDs for id and version fields
7. Set created timestamp to import time
8. Leave deleted as NULL for active products