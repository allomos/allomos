// Whether the tab a part of the window is in is the one in front (Task-A01-marble320): keyboard
// shortcuts and menu commands go to the tab in front only. True outside tabs (the companion page).

import { createContext } from "react";

export const TabActive = createContext(true);
