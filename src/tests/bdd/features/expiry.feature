Feature: Optional expiry (TTL) for state entries

  Scenario: Persisting a task result with a TTL S1
    Given a TTL of 2 seconds is configured for state entries S1
    And a task result "Hello, world!" is ready to be persisted S1
    When the task result is serialized and persisted to the state store S1
    Then the persisted entry should exist in the state store immediately S1
    And after 3 seconds, the entry should be expired and no longer available S1

  Scenario: Persisting a task result without a TTL
    Given no TTL is configured for state entries S2
    And a task result "Hello, world!" is ready to be persisted S2
    When the task result is serialized and persisted to the state store S2
    Then the persisted entry should exist in the state store S2
    And the entry should not expire automatically S2