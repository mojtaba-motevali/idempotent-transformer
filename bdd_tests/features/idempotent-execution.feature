Feature:  Idempotent Execution Wrapper
The library shall wrap any asynchronous task and guarantee that the original task is not malformed or mutated, returning cached results for subsequent calls.

    Scenario: Successful execution of a task with cached results
        Given an asynchronous task that returns a value
        When I wrap the task with the idempotent execution wrapper
        And I execute the wrapped task
        Then the task should execute successfully and the result should be returned
        When I execute the wrapped task again
        Then the task should not be executed again and the cached result should be returned

    Scenario: Workflow Idempotency
        Given a workflow including 4 tasks
        When I execute 2 tasks successfully and third one fails
        Then I retry execution of the all tasks  
        Then the task should execute successfully and all tasks should have been executed only once
    
    Scenario: Workflow within a workflow
        Given a workflow that includes multiple tasks that one of the includes multiple other tasks
        When I execute second task of the inner workflow and fails
        Then I retry the outer workflow to recover from the failure
        Then the already successfully executed tasks must not re-execute and outer workflow must successfully finish.
        