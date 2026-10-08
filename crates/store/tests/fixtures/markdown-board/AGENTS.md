# Bakery site — working guide

## Boundaries

- Never print or commit credentials.
- Deployments need the owner's approval, recorded on the card.

## Working procedure

- **Status lives on the card.** Each card's board-fields table (State, Done, Depends on, Waiting on) is the only source; run `node tools/board.mjs --write` after changing a card to regenerate the board.
- **Claiming protocol.** Write your session's name into the card's Session field and into the session registry on the task board, in a commit of its own.
- **Labels**: implemented → locally verified → deployed and verified → owner verified.
