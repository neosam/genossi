## ADDED Requirements

### Requirement: Retry endpoint logging

The system SHALL log every call to `POST /api/mail/jobs/{id}/retry` with the job ID at INFO level before processing the retry.

#### Scenario: Retry endpoint called
- **WHEN** `POST /api/mail/jobs/{id}/retry` is called
- **THEN** the system logs an INFO message containing the job ID before executing the retry logic

### Requirement: Job completion update resilience

The system SHALL retry the job status update up to 3 times when the completion check determines the job is done or failed. If all retries fail, the system SHALL log an ERROR with the job ID and continue processing.

#### Scenario: Job update succeeds on first try
- **WHEN** the worker determines a job is complete and the database update succeeds
- **THEN** the job status is updated in the database

#### Scenario: Job update fails transiently
- **WHEN** the worker determines a job is complete and the first database update fails but the second succeeds
- **THEN** the job status is updated in the database after the retry

#### Scenario: Job update fails permanently
- **WHEN** the worker determines a job is complete and all 3 update attempts fail
- **THEN** the system logs an ERROR with the job ID and the worker continues to the next iteration
