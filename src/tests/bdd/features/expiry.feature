Feature: Optional expiry (TTL) for state entries

  Scenario: Persisting a task result with a TTL
    Given a TTL of 2 seconds is configured for state entries
    And a task result "Hello, world!" is ready to be persisted
    When the task result is serialized and persisted to the state store
    Then the persisted entry should exist in the state store immediately
    And after 3 seconds, the entry should be expired and no longer available

  Scenario: Persisting a task result without a TTL
    Given no TTL is configured for state entries
    And a task result "Hello, world!" is ready to be persisted
    When the task result is serialized and persisted to the state store
    Then the persisted entry should exist in the state store
    And the entry should not expire automatically