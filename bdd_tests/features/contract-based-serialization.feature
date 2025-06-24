Feature: Contract Based Serialization

  Scenario: Serialize a user-defined class as a task result
    Given a user-defined class with private attributes and a task that returns an instance of this class
    When the task is executed twice to showcase a retry operation
    Then its result in second execution is decoded back to an instance of the class without the need any method to be called by developer