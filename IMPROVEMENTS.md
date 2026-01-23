# Improvements over C htop

This document outlines intentional improvements and enhancements made in htop-rs compared to the original C htop implementation.

## User Interface

### Filtered Mode Selection Color

When using the F4 Filter function, the selected row now displays with a distinct yellow/follow color to indicate that filtering is active. This provides better visual feedback to the user that they are in filter mode, making it easier to distinguish between normal selection and filtered selection states.

In C htop, the selection color remains the same regardless of filter state, which can make it unclear whether a filter is currently active.
