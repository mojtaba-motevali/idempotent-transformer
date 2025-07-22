Feature: Optional expiry (TTL) for state entries

  Scenario: Persisting a task result with a TTL 
    Given a TTL of 5 seconds is configured for workflow completion data state 
    When a task is executed and workflow is completed
    Then wait for 10 seconds and the workflow must not be found
