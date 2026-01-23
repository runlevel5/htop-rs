# Improvements over C htop

This document outlines intentional improvements and enhancements made in htop-rs compared to the original C htop implementation.

## User Interface

### Filtered Mode Visual Indicator

When using the F4 Filter function, htop-rs provides enhanced visual feedback:

- **Yellow header background**: The column header row turns yellow when a filter is active, making it immediately clear that the process list is filtered
- **Normal cyan selection**: The selected row uses the standard cyan selection color, keeping the selection consistent with non-filtered mode

This design choice uses the header color change as the primary indicator of filter state, rather than changing the selection color. This provides clear visual feedback without making the selection look different from normal operation.

In C htop, there is no visual indication in the header that a filter is active, which can make it unclear whether the process list is currently filtered.
