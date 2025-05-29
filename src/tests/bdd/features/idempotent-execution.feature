Feature:  Idempotent Execution Wrapper
The library shall wrap any asynchronous task and guarantee that the original task is not malformed or mutated, returning cached results for subsequent calls.

    Scenario: Successful execution of a task with cached results
        Given an asynchronous task that returns a value
        When I wrap the task with the idempotent execution wrapper
        And I execute the wrapped task
        Then the task should execute successfully and the result should be returned
        When I execute the wrapped task again
        Then the task should not be executed again and the cached result should be returned

    Scenario: Task execution with error handling
        Given an asynchronous task that throws an error
        When I wrap the task with the idempotent execution wrapper
        And I execute the wrapped task
        Then the error should be propagated
        When I execute the wrapped task again
        Then the error should be propagated again
        And the task should be executed each time

    Scenario: Task execution with different parameters
        Given an asynchronous task that accepts parameters
        When I wrap the task with the idempotent execution wrapper
        And I execute the wrapped task with parameter "A"
        Then the task should execute with parameter "A"
        And the result should be cached for parameter "A"
        When I execute the wrapped task with parameter "B"
        Then the task should execute throw Conflict exception

    Scenario: Task execution with timeout
        Given an asynchronous task that takes longer than the timeout period
        When I wrap the task with the idempotent execution wrapper
        And I set a timeout period
        And I execute the wrapped task
        Then the task should be interrupted after the timeout period
        And a timeout error should be returned
