// SPDX-License-Identifier: GPL-3.0-only
/* Small ABI boundary. Game data remains owned by the original DOOM engine. */
#include "doomstat.h"
#include "d_items.h"
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <setjmp.h>
#include <stdlib.h>
#include "doomgeneric.h"
#include "i_system.h"
#include "r_local.h"
#include "m_menu.h"
#include "g_game.h"

extern void M_ClearMenus(void);
void MCDU_HostAction(int action) {
    M_ClearMenus();
    if (action == 1) {
        paused = false;
        G_DeferedInitNew(sk_medium, 1, 1);
    } else if (action == 2) {
        paused = true;
    } else if (action == 3) {
        paused = false;
    }
}
#include "doom_bridge.h"

static McduText mcdu_text;
void MCDU_BeginText(void) {
    memset(&mcdu_text, 0, sizeof(mcdu_text));
}

void MCDU_TextMenu(int kind) {
    mcdu_text.count = 0;
    mcdu_text.menu = kind;
}

void MCDU_AddText(int x, int y, int color, const char *text) {
    if (!text || !*text || mcdu_text.count >= MCDU_TEXT_LINES) {
        return;
    }
    McduTextLine *line = &mcdu_text.lines[mcdu_text.count++];
    line->x = mcdu_text.menu == 2 ? -1 : x;
    line->y = y;
    line->color = color;
    snprintf(line->text, sizeof(line->text), "%s", text);
}
void MCDU_GetText(McduText *out) {
    memcpy(out, &mcdu_text, sizeof(*out));
}

extern int screenSize;
void MCDU_EnsureView(void) {
    /* The glyph renderer consumes the complete 320x168 gameplay viewport.
       Keep the status bar and prevent +/- or a saved config shrinking it. */
    if (screenblocks != 10 || detailLevel != 0) {
        screenblocks = 10;
        screenSize = 7;
        detailLevel = 0;
        R_SetViewSize(10, 0);
    }
}

/* Pixel ownership follows the clipped painter order, including masked walls
   and foreground objects. It never changes the original framebuffer. */
static uint8_t mcdu_mask[SCREENWIDTH * SCREENHEIGHT];
extern int viewwindowx, viewwindowy;
void MCDU_ClearMask(void) {
    memset(mcdu_mask, 0, sizeof(mcdu_mask));
}

const uint8_t *MCDU_GetMask(void) {
    return mcdu_mask;
}

void MCDU_MarkColumn(int kind, int x, int y1, int y2) {
    int x0 = viewwindowx + (x << detailshift);
    int span = 1 << detailshift;
    for (int y = y1; y <= y2; ++y) {
        int sy = viewwindowy + y;
        if (sy < 0 || sy >= SCREENHEIGHT) {
            continue;
        }
        for (int dx = 0; dx < span; ++dx) {
            if (x0 + dx >= 0 && x0 + dx < SCREENWIDTH) {
                mcdu_mask[sy * SCREENWIDTH + x0 + dx] = (uint8_t)kind;
            }
        }
    }
}

/* Escape only C stack frames. No Rust callback is live when DOOM quits/errors. */
static jmp_buf engine_exit;
void MCDU_Exit(int status) {
    longjmp(engine_exit, status == 0 ? 1 : 2);
}

int MCDU_Create(int argc, char **argv) {
    int status = setjmp(engine_exit);
    if (status == 0) {
        doomgeneric_Create(argc, argv);
    }
    return status;
}
int MCDU_Tick(void) {
    int status = setjmp(engine_exit);
    if (status == 0) {
        doomgeneric_Tick();
    }
    return status;
}
int MCDU_Shutdown(void) {
    int status = setjmp(engine_exit);
    if (status == 0) {
        I_Quit();
    }
    free(DG_ScreenBuffer);
    DG_ScreenBuffer = NULL;
    fflush(NULL);
    return status;
}

void MCDU_GetHud(McduHud *out) {
    memset(out, 0, sizeof(*out));
    out->active = gamestate == GS_LEVEL;
    if (!out->active) {
        return;
    }
    player_t *p = &players[consoleplayer];
    out->health = p->health;
    out->armor = p->armorpoints;
    out->weapon = (int32_t)p->readyweapon;
    ammotype_t ammo = weaponinfo[p->readyweapon].ammo;
    out->ammo = ammo == am_noammo ? -1 : p->ammo[ammo];
    out->damage = p->damagecount;
    for (int i = 0; i < 4; ++i) {
        out->ammo_counts[i] = p->ammo[i];
        out->ammo_max[i] = p->maxammo[i];
    }
    for (int i = 0; i < NUMWEAPONS; ++i) {
        if (p->weaponowned[i]) {
            out->owned |= 1 << i;
        }
    }
    for (int i = 0; i < 3; ++i) {
        if (p->cards[i] || p->cards[i + 3]) out->keys |= 1 << i;
        if (p->cards[i + 3]) out->skulls |= 1 << i;
    }
}
