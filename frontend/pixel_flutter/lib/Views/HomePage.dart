import 'dart:convert';

import 'package:flutter/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import 'package:web_socket_channel/io.dart';

import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkEvent.dart';
import 'package:pixel_flutter/blocs/UserBlocs.dart';
import 'package:pixel_flutter/blocs/VerticalTabBlocs.dart';
import 'package:pixel_flutter/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter/components/Background/GeneralBackground.dart';
import 'package:pixel_flutter/components/Menu/UserDrawer.dart';
import 'package:pixel_flutter/components/NavigationBar/VerticalTab/VerticalTabText.dart';
import 'package:pixel_flutter/components/Categories/CategoryHeader.dart';
import 'package:pixel_flutter/components/Categories/CategoryList.dart';
import 'package:pixel_flutter/components/Button/AddPostButton.dart';
import 'package:pixel_flutter/components/NavigationBar/CategoryNavBar.dart';

import 'package:pixel_flutter/Views/TalkPage.dart';

import 'package:pixel_flutter/models/Talk.dart';

import 'package:pixel_flutter/style/colors.dart';
import 'package:pixel_flutter/style/text.dart';

import '../env.dart';

class HomePage extends StatefulWidget with env {
  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  ErrorBloc _errorBloc;
  WebSocketChannel channel;

  @override
  void initState() {
    _errorBloc = BlocProvider.of<ErrorBloc>(context);
    channel = IOWebSocketChannel.connect(widget.WS_URL);
    channel.stream.listen((msg) => handleMessage(msg: msg));
    super.initState();
  }

  @override
  void dispose() {
    _errorBloc.dispatch(HideSnack());
    channel.sink.close();
    super.dispose();
  }

  Future<void> handleUserState(UserState state) async {
    if (state is UserLoaded) {
      final String auth = '/auth ' + state.user.token;
      channel.sink.add(auth);
      return;
    }
  }

  void getTalks(int talkId) {
    channel.sink.add(GetTalks(talkId: talkId).toJSON());
  }

  Future<void> handleMessage({String msg}) async {
    if (msg.startsWith('/')) {
      final str = msg.split(" ").toList();
      if (str.length != 2) {
        return;
      }
      if (str[0] == "/talks") {
        final data = jsonDecode(str[1]) as List;
        final result = data.map((rawTalk) {
          return Talk(
              id: rawTalk['id'],
              name: rawTalk['name'],
              description: rawTalk['description'],
              privacy: rawTalk['privacy'],
              owner: rawTalk['owner'],
              admin: rawTalk['admin'].cast<int>(),
              users: rawTalk['users'].cast<int>());
        }).toList();
        BlocProvider.of<TalkBloc>(context).dispatch(GotTalk(talks: result));
      }
    }
  }

  Future<bool> _onWillPop() {
    return showDialog(
        context: context,
        builder: (context) => AlertDialog(
              title: Text('Leaving?'),
              content: Text('Do you want to exit the app'),
              actions: <Widget>[
                FlatButton(
                  onPressed: () => Navigator.pop(context, false),
                  child: Text(
                    'Cancel',
                    style: recoverButtonStyle,
                  ),
                ),
                RaisedButton(
                  color: primaryColor,
                  onPressed: () => Navigator.pop(context, true),
                  child: Text(
                    'Confirm',
                    style: submitButtonStyle.copyWith(fontSize: 16),
                  ),
                )
              ],
            ));
  }

  @override
  Widget build(BuildContext context) {
    return WillPopScope(
        onWillPop: _onWillPop,
        child: Scaffold(
            endDrawer: UserDrawer(),
            body: MultiBlocListener(
              listeners: [
                // listen to userState and trigger web socket connection
                BlocListener<UserEvent, UserState>(
                    bloc: BlocProvider.of<UserBloc>(context),
                    listener: (BuildContext context, UserState state) =>
                        handleUserState(state)),
                BlocListener<ErrorEvent, ErrorState>(
                    bloc: _errorBloc,
                    listener: (BuildContext context, ErrorState state) async {
                      if (state is NoSnack) {
                        Scaffold.of(context).hideCurrentSnackBar();
                      } else if (state is ShowSuccess) {
                        Scaffold.of(context).showSnackBar(SnackBar(
                          duration: Duration(seconds: 2),
                          backgroundColor: Colors.green,
                          content: Text(state.success),
                        ));
                      } else if (state is ShowError) {
                        Scaffold.of(context).showSnackBar(SnackBar(
                          duration: Duration(seconds: 2),
                          backgroundColor: Colors.deepOrangeAccent,
                          content: Text(state.error),
                        ));
                      }
                    }),
              ],
              child: BlocBuilder(
                  bloc: BlocProvider.of<VerticalTabBloc>(context),
                  builder: (BuildContext context, VerticalTabState tabState) {
                    if (tabState is Selected) {
                      return Stack(
                        children: <Widget>[
                          GeneralBackground(),
                          Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: <Widget>[
                                CatNavBar(),
                                CategoryHeader(
                                  tabIndex: tabState.index,
                                ),
                                Spacer(),
                                InkWell(
                                    onTap: () => getTalks(1),
                                    child: AddPostButton(text: 'New Topic'))
                              ]),
                          Center(
                              child: Container(
                                  height: 470,
                                  child: CardStack(
                                    selectedTabIndex: tabState.index,
                                  )))
                        ],
                      );
                    } else {
                      return Container();
                    }
                  }),
            )));
  }
}

class CardStack extends StatefulWidget {
  final int selectedTabIndex;

  CardStack({this.selectedTabIndex});

  @override
  _CardStackState createState() => _CardStackState();
}

class _CardStackState extends State<CardStack>
    with SingleTickerProviderStateMixin {
  AnimationController _animationController;
  Animation<double> _animationDouble;

  initAnimation() {
    _animationController.reset();
    _animationController.forward();
  }

  @override
  void initState() {
    _animationController =
        AnimationController(vsync: this, duration: Duration(milliseconds: 500));
    _animationDouble =
        Tween<double>(begin: 0.0, end: 1.0).animate(_animationController);
    super.initState();
  }

  @override
  void dispose() {
    _animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: <Widget>[
        Positioned(
          left: -20,
          top: 0,
          bottom: 0,
          width: 100,
          child: Padding(
            padding: EdgeInsets.symmetric(vertical: 80.0),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: <Widget>[
                VerticalTabText(
                  text: 'Hot',
                  index: 0,
                ),
                VerticalTabText(
                  text: 'Game',
                  index: 1,
                ),
                VerticalTabText(
                  text: 'Talk',
                  index: 2,
                ),
              ],
            ),
          ),
        ),
        FutureBuilder(
          future: initAnimation(),
          builder: (context, snapshot) {
            return FadeTransition(
              opacity: _animationDouble,
              child: Padding(
                  padding: EdgeInsets.only(left: 60),
                  // ToDo: make separate class for list view on different tabs.
                  child: widget.selectedTabIndex == 0
                      ? CategoryList()
                      : widget.selectedTabIndex == 1
                          ? CategoryList()
                          : TalkPage()),
            );
          },
        ),
      ],
    );
  }
}
