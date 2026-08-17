from textual.message import Message


class PlayerConnected(Message):
    def __init__(self, player_index: int, player_id: str) -> None:
        super().__init__()
        self.player_index = player_index
        self.player_id = player_id


class PlayerStateUpdated(Message):
    def __init__(self, player_index: int, field: str, value: object) -> None:
        super().__init__()
        self.player_index = player_index
        self.field = field
        self.value = value


class PlayerFundsChanged(Message):
    def __init__(self, player_index: int, new_funds: int) -> None:
        super().__init__()
        self.player_index = player_index
        self.new_funds = new_funds


class GameEvent(Message):
    def __init__(self, player_index: int, text: str) -> None:
        super().__init__()
        self.player_index = player_index
        self.text = text


class SessionStopped(Message):
    def __init__(self, player_index: int, reason: str) -> None:
        super().__init__()
        self.player_index = player_index
        self.reason = reason


class CommunityCardsUpdated(Message):
    def __init__(self, text: str) -> None:
        super().__init__()
        self.text = text
