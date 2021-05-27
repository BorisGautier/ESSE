import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'package:esse/l10n/localizations.dart';
import 'package:esse/provider.dart';

import 'package:esse/apps/assistant/page.dart';
import 'package:esse/apps/file/page.dart';
import 'package:esse/apps/group_chat/page.dart';


enum InnerService {
  Files,
  Assistant,
  GroupChat,
}

extension InnerServiceExtension on InnerService {
  List<String> params(AppLocalizations lang) {
    switch (this) {
      case InnerService.Files:
        return [lang.files, lang.filesBio, 'assets/logo/logo_files.png'];
      case InnerService.Assistant:
        return [lang.assistant, lang.assistantBio, 'assets/logo/logo_assistant.png'];
      case InnerService.GroupChat:
        return [lang.groupChat, lang.groupChatBio, 'assets/logo/logo_group_chat.png'];
    }
  }

  void callback(context, isDesktop, lang) {
    Widget coreWidget = null;
    String listTitle = null;
    Widget listHome = null;

    if (isDesktop) {
      switch (this) {
        case InnerService.Files:
          listTitle = lang.files;
          listHome = FolderList();
          break;
        case InnerService.Assistant:
          coreWidget = AssistantDetail();
          break;
        case InnerService.GroupChat:
          listTitle = lang.groupChat;
          listHome = GroupChatList();
          break;
      }
      Provider.of<AccountProvider>(context, listen: false).updateActivedWidget(coreWidget, listTitle, listHome);
    } else {
      switch (this) {
        case InnerService.Files:
          Provider.of<AccountProvider>(context, listen: false).updateActivedWidget(null, lang.files, FolderList());
          break;
        case InnerService.Assistant:
          Navigator.push(context, MaterialPageRoute(builder: (_) => AssistantPage()));
          break;
        case InnerService.GroupChat:
          Provider.of<AccountProvider>(context, listen: false).updateActivedWidget(null, lang.groupChat, GroupChatList());
          break;
      }
    }
  }
}
