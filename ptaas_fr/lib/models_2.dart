import 'package:json_annotation/json_annotation.dart';

part 'models_2.g.dart';

// dart run build_runner build

@JsonSerializable(genericArgumentFactories: true)
class APIResponse<T> {
  final T? processed;
  final bool? missingToken;
  final bool? emptyToken;
  final bool? notLoggedIn;
  final bool? internalServerError;

  APIResponse({
    this.processed,
    this.missingToken,
    this.emptyToken,
    this.notLoggedIn,
    this.internalServerError,
  });

  factory APIResponse.fromJson(
    Map<String, dynamic> json,
    T Function(Object? json) fromJsonT,
  ) =>
      _$APIResponseFromJson(json, fromJsonT);

  Map<String, dynamic> toJson(Object? Function(T value) toJsonT) =>
      _$APIResponseToJson(this, toJsonT);
}

@JsonSerializable()
class AllProjectsResponse {
  final AllProjectsResponseProcessed? processed;
  final bool? cantReadProjects;
  final bool? aProjectIsMissing;

  AllProjectsResponse({
    this.processed,
    this.cantReadProjects,
    this.aProjectIsMissing,
  });

  factory AllProjectsResponse.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseToJson(this);
}

@JsonSerializable()
class AllProjectsResponseProcessed {
  final List<Project> projects;

  AllProjectsResponseProcessed({required this.projects});

  factory AllProjectsResponseProcessed.fromJson(Map<String, dynamic> json) =>
      _$AllProjectsResponseProcessedFromJson(json);

  Map<String, dynamic> toJson() => _$AllProjectsResponseProcessedToJson(this);
}

@JsonSerializable()
class Script {
  final String id;

  Script({required this.id});

  factory Script.fromJson(Map<String, dynamic> json) => _$ScriptFromJson(json);

  Map<String, dynamic> toJson() => _$ScriptToJson(this);
}

@JsonSerializable()
class Project {
  final String id;
  final bool installed;
  final List<Script> scripts;

  Project({required this.id, required this.installed, required this.scripts});

  factory Project.fromJson(Map<String, dynamic> json) =>
      _$ProjectFromJson(json);

  Map<String, dynamic> toJson() => _$ProjectToJson(this);
}
