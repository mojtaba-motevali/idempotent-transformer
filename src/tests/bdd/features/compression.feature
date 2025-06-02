Feature: Optional compression of serialized task results

  Scenario: Persisting a task result with compression enabled
    Given compression is enabled
    And a task result "Hello, world!" is ready to be persisted
    When the task result is serialized and persisted to the state store
    Then the persisted data should be compressed
    And the original task result should not be stored in plain text

  Scenario: Retrieving a compressed task result
    Given compression is enabled in the library configuration
    And a compressed task result is stored in the state store
    When the task result is retrieved from the state store and decompress the data transparently
    Then retrieved result should match the original task's result.

  Scenario: Backward compatibility with existing uncompressed data
    Given compression is now enabled in the library configuration
    And an uncompressed task result was previously stored in the state store
    When the task result is retrieved from the state store and should correctly retrieve the uncompressed result
    Then retrieved result should match the original task's result

  Scenario: Backward compatibility with existing compressed data
    Given the compression was enabled and now is disabled and a compressed task result was previously stored in the state store
    When the task result is retrieved with compression disabled from the state store, it should correclty be decompressed
    Then retrieved result must match the original task's result.
    