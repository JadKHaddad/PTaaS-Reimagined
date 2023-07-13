// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'msg.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

WSFromClient _$WSFromClientFromJson(Map<String, dynamic> json) => WSFromClient(
      subscribe: json['Subscribe'] == null
          ? null
          : SubscribeMessage.fromJson(
              json['Subscribe'] as Map<String, dynamic>),
      unsubscribe: json['Unsubscribe'] == null
          ? null
          : UnsubscribeMessage.fromJson(
              json['Unsubscribe'] as Map<String, dynamic>),
    );

Map<String, dynamic> _$WSFromClientToJson(WSFromClient instance) =>
    <String, dynamic>{
      'Subscribe': instance.subscribe,
      'Unsubscribe': instance.unsubscribe,
    };

SubscribeMessage _$SubscribeMessageFromJson(Map<String, dynamic> json) =>
    SubscribeMessage(
      project_id: json['project_id'] as String?,
    );

Map<String, dynamic> _$SubscribeMessageToJson(SubscribeMessage instance) =>
    <String, dynamic>{
      'project_id': instance.project_id,
    };

UnsubscribeMessage _$UnsubscribeMessageFromJson(Map<String, dynamic> json) =>
    UnsubscribeMessage(
      project_id: json['project_id'] as String?,
    );

Map<String, dynamic> _$UnsubscribeMessageToJson(UnsubscribeMessage instance) =>
    <String, dynamic>{
      'project_id': instance.project_id,
    };
