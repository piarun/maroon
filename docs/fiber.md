# Fiber

Fiber is a single execution flow entity. We can think about it as an Actor that runs some business logic.
Fibers are run logically independently and concurrently. However the whole engine is single-threaded, so in reality there will be no parallel work(at least now)

## features/statements
- there is one root fiber(named root) that is started automatically by runtime
- fibers have internal states
    - heap, stack
    - internal states can't be accessed directly by other fibers, only through async messages
- other fibers are started by previously created fibers
    - TBD: fiber shares/moves some part of compute quota with/to the ones they created
    - [?] if parent 'dies' - what happens with children? what happens with compute quota?
- fibers can create queues for communications
	- [?] do gateways know about queues? Probably, at least about some of them for a better routing, or maybe queues should be registered somewhere so gateways start to know about them
	- queues are managed & owned by runtime
        - [?] should all maroon-queues be 'public' for other fibers?
- fibers start their lifecycle from 'main' function
- fibers can finish it's work and being 'destroyed'
    - heap and stack 'disappears' in that case and can't be accessed anymore
    - fibers don't return anything(synchronously, like function returns value, only through some async tools)
	    - [?] how do they return results in that case? and I'm talking about external tasks, not cross-fiber communication. Because for cross-fiber it's clear: async-queues
	        - [?] probably/maybe there should be some special(from runtime perspective) 'response/results' queue where fibers will be passing result+some metadata on for which task it was?
            - [?] or maybe just putting the result into the Future object? If it was the message from some other fiber - that fiber will be awaken, if from runtime - it means there will be a result
