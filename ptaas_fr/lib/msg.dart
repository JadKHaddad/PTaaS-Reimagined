import 'package:json_annotation/json_annotation.dart';

part 'msg.g.dart';

@JsonSerializable()
class WSFromClient {
  @JsonKey(name: 'Subscribe')
  SubscribeMessage? subscribe;
  @JsonKey(name: 'Unsubscribe')
  UnsubscribeMessage? unsubscribe;

  WSFromClient({this.subscribe, this.unsubscribe});

  factory WSFromClient.fromJson(Map<String, dynamic> json) =>
      _$WSFromClientFromJson(json);
  Map<String, dynamic> toJson() => _$WSFromClientToJson(this);
}

@JsonSerializable()
class SubscribeMessage {
  String? project_id;

  SubscribeMessage({this.project_id});

  factory SubscribeMessage.fromJson(Map<String, dynamic> json) =>
      _$SubscribeMessageFromJson(json);
  Map<String, dynamic> toJson() => _$SubscribeMessageToJson(this);
}

@JsonSerializable()
class UnsubscribeMessage {
  String? project_id;

  UnsubscribeMessage({this.project_id});

  factory UnsubscribeMessage.fromJson(Map<String, dynamic> json) =>
      _$UnsubscribeMessageFromJson(json);
  Map<String, dynamic> toJson() => _$UnsubscribeMessageToJson(this);
}
