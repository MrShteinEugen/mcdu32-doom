// SPDX-License-Identifier: GPL-3.0-only
/* Host ABI shared by the bridge and the small hooks in vendored DOOM.
   Rust mirrors these fixed-width layouts in display/hud.rs and doom/text.rs. */
#ifndef MCDU_DOOM_BRIDGE_H
#define MCDU_DOOM_BRIDGE_H
#include <stdint.h>

#define MCDU_TEXT_LINES 32
#define MCDU_TEXT_BYTES 128

typedef struct {
    int32_t active, health, armor, ammo, weapon, owned, keys, skulls, damage;
    int32_t ammo_counts[4], ammo_max[4];
} McduHud;

typedef struct {
    int32_t x, y, color;
    char text[MCDU_TEXT_BYTES];
} McduTextLine;

typedef struct {
    int32_t count, menu;
    McduTextLine lines[MCDU_TEXT_LINES];
} McduText;

/* Keep failures at the ABI boundary visible even with older C compilers. */
typedef char McduHudLayoutCheck[sizeof(McduHud) == 68 ? 1 : -1];
typedef char McduTextLayoutCheck[sizeof(McduText) == 4488 ? 1 : -1];

int MCDU_Create(int argc, char **argv);
int MCDU_Tick(void);
int MCDU_Shutdown(void);
/* 1 starts a new game, 2 pauses for host UI, 3 resumes a paused game. */
void MCDU_HostAction(int action);
void MCDU_GetHud(McduHud *out);
void MCDU_GetText(McduText *out);
const uint8_t *MCDU_GetMask(void);
void MCDU_ClearMask(void);
void MCDU_MarkColumn(int kind, int x, int y1, int y2);
void MCDU_BeginText(void);
void MCDU_AddText(int x, int y, int color, const char *text);
void MCDU_TextMenu(int kind);
void MCDU_EnsureView(void);
#endif
