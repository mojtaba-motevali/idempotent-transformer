Feature: Optional compression of serialized task results

  Scenario: Persisting a task result with compression enabled
    Given compression is enabled in the library configuration
    And a task result "Hello, world!" is ready to be persisted
    When the task result is serialized and persisted to the state store
    Then the persisted data should be compressed
    And the original task result should not be stored in plain text

  Scenario: Retrieving a compressed task result
    Given compression is enabled in the library configuration
    And a compressed task result is stored in the state store
    When the task result is retrieved from the state store
    Then the library should decompress the data transparently
    And the retrieved result should match the original task result

  Scenario: Persisting a task result with compression disabled
    Given compression is disabled in the library configuration
    And a task result "Hello, world!" is ready to be persisted
    When the task result is serialized and persisted to the state store
    Then the persisted data should not be compressed
    And the original task result should be stored in plain text

  Scenario: Retrieving an uncompressed task result
    Given compression is disabled in the library configuration
    And an uncompressed task result is stored in the state store
    When the task result is retrieved from the state store
    Then the library should not attempt to decompress the data
    And the retrieved result should match the original task result

  Scenario: Backward compatibility with existing uncompressed data
    Given compression is now enabled in the library configuration
    And an uncompressed task result was previously stored in the state store
    When the task result is retrieved from the state store
    Then the library should correctly retrieve the uncompressed result
    And the retrieved result should match the original task result