# rong_worker

Worker APIs for the Rong JavaScript runtime.

This crate provides worker-oriented JavaScript bindings for Rong, including the
runtime pieces needed to create and coordinate long-lived worker execution from
JavaScript.

Enable the matching engine feature (`quickjs` or `jscore`) when depending on
this crate directly.
