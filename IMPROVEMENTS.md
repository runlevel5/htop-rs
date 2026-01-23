# Improvements over C htop

This document outlines intentional improvements and enhancements made in htop-rs compared to the original C htop implementation.

## User Interface

### Search and Filter Mode Visual Indicator

When using F3 Search or F4 Filter functions, htop-rs provides enhanced visual feedback:

- **Yellow header background**: The column header row turns yellow when search or filter is active, making it immediately clear that the view is in a special mode

For F4 Filter mode specifically:
- **Normal cyan selection**: The selected row uses the standard cyan selection color, since the yellow header already indicates filtered state

For F3 Search mode:
- **Yellow "following" selection**: The selected row uses the yellow selection color to indicate the search match is being followed (same as C htop)

In C htop, there is no visual indication in the header that a search or filter is active, which can make it unclear whether the process list is currently filtered or a search is in progress.
