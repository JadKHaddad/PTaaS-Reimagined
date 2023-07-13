import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'models.dart';
import 'msg.dart';

void main() {
  runApp(const MainApp());
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

  void _doSomeAPIStuff() {
    String allProjectsResponseSuccessString =
        '{"success":true,"responseType":"allProjectsResponse","data":{"projects":[{"id":"project1","installed":true,"scripts":[{"id":"script1"},{"id":"script2"}]},{"id":"project2","installed":false,"scripts":[{"id":"script3"}]}]},"error":null}';
    String allProjectsResponseErrorString =
        '{"success":false,"responseType":"allProjectsResponse","data":null,"error":{"errorType":"cantReadProjects","errorMessage":"Failed to read projects."}}';
    String apiErrorResponseString =
        '{"success":false,"responseType":"gerneralResponse","data":null,"error":{"errorType":"aPIKeyIsMissing","errorMessage":"API key is missing."}}';

    Map<String, dynamic> json = jsonDecode(apiErrorResponseString);

    try {
      // if this one fails, the error is in an api error so we parse it as such
      APIResponse<AllProjectsResponseData?, AllProjectsResponseErrorType?>
          allProjectsResponse = APIResponse.fromJson(
              json,
              (json) => AllProjectsResponseData.fromJson(
                  json as Map<String, dynamic>),
              (json) =>
                  AllProjectsResponseErrorType.fromString(json as String));
      print("All projects response: $json");
    } catch (e) {
      APIResponse<Object?, APIGerneralResponseErrorType?> apiErrorResponse =
          APIResponse.fromJson(
              json,
              (json) =>
                  null /* we don't care about the data because its never there!*/,
              (json) =>
                  APIGerneralResponseErrorType.fromString(json as String));
      print("API error response: $json");
    }
  }

  void _doOtherAPIStuff() {
    String wsMessage = '{"Subscribe":{"project_id":"project1"}}';
    // we already know the type of the message 'WSFromClient', so we can just parse it as such
    Map<String, dynamic> json = jsonDecode(wsMessage);
    WSFromClient fromClient = WSFromClient.fromJson(json);
    print("WS message: $fromClient");
    print("sub: ${fromClient.subscribe}, unsub: ${fromClient.unsubscribe}");
    // is this a subscribe message? or something else?
    if (fromClient.subscribe != null) {
      print("Subscribe message: ${fromClient.subscribe}");
    }

    // is this a unsubscribe message? or something else?
    if (fromClient.unsubscribe != null) {
      print("Unsubscribe message: ${fromClient.unsubscribe}");
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Center(
            child: Column(
          children: [
            TextButton(
                onPressed: _doSomeAPIStuff,
                child: const Text("Do some API stuff")),
            TextButton(
                onPressed: _doOtherAPIStuff,
                child: const Text("Do other API stuff")),
          ],
        )),
      ),
    );
  }
}
