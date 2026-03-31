## MODIFIED Requirements

### Requirement: Excel import balance conversion
The Excel import SHALL convert the "Guthaben aktuell" value from Euro to Cent by multiplying by 100 before storing as `current_balance`.

#### Scenario: Integer Euro value
- **WHEN** the Excel contains a balance value of 150
- **THEN** the system stores `current_balance` as 15000 (cents)

#### Scenario: Decimal Euro value
- **WHEN** the Excel contains a balance value of 150.50
- **THEN** the system stores `current_balance` as 15050 (cents)

#### Scenario: Zero or empty balance
- **WHEN** the Excel contains an empty or zero balance
- **THEN** the system stores `current_balance` as 0
