import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicBloc.dart';

import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/components/UserButton.dart';
import 'package:pixel_flutter_web/components/TopicTile.dart';
import 'package:pixel_flutter_web/components/PostTile.dart';
import 'package:pixel_flutter_web/components/UserDrawer.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';

import 'package:pixel_flutter_web/models/Topic.dart';

class TopicPage extends StatefulWidget {
  final Topic topic;

  TopicPage({Key key, @required this.topic}) : super(key: key);

  @override
  _TopicPageState createState() => _TopicPageState();
}

class _TopicPageState extends State<TopicPage> {
  TopicBloc _topicBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    _topicBloc = TopicBloc();
    _topicBloc.dispatch(GetTopic(topicId: widget.topic.id));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: BlocProvider.of<UserBloc>(context),
        builder: (context, userState) {
          return Scaffold(
            endDrawer: userState is UserLoaded ? UserDrawer() : null,
            appBar: AppBar(
              elevation: 5.0,
              title: Text(widget.topic.title),
              leading: IconButton(
                onPressed: () => Navigator.pop(context),
                icon: Icon(Icons.arrow_back),
              ),
              actions: <Widget>[UserButton()],
            ),
            body: BlocListener(
              bloc: BlocProvider.of<ErrorBloc>(context),
              listener: (BuildContext context, ErrorState state) async {
                snackbarController(context, state);
              },
              child:
                  Layout(topicBloc: _topicBloc, controller: _scrollController),
            ),
          );
        });
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(GetTopic(topicId: widget.topic.id));
    }
  }

  snackbarController(BuildContext context, ErrorState state) async {
    if (state is NoSnack) {
      Scaffold.of(context).hideCurrentSnackBar();
    } else if (state is ShowSuccess) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.green,
        content: Text(
          state.success,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    } else if (state is ShowError) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        backgroundColor: Colors.deepOrangeAccent,
        content: Text(
          state.error,
          textAlign: TextAlign.center,
          style: TextStyle(fontSize: 25, fontWeight: FontWeight.bold),
        ),
      ));
    }
  }
}

class Layout extends StatelessWidget with env {
  final TopicBloc topicBloc;
  final ScrollController controller;

  Layout({this.topicBloc, this.controller});

  @override
  Widget build(BuildContext context) {
    return Row(mainAxisAlignment: MainAxisAlignment.center, children: <Widget>[
      Container(
        width: MediaQuery.of(context).size.width > 700
            ? 700
            : MediaQuery.of(context).size.width,
        child: BlocBuilder(
            bloc: topicBloc,
            builder: (context, state) {
              if (state is TopicLoaded) {
                print(state.posts);
                return Column(
                  children: <Widget>[
                    TopicTile(topic: state.topic),
                    state.posts.isEmpty
                        ? Container()
                        : Expanded(
                            child: ListView.builder(
                              controller: controller,
                              itemCount: state.hasReachedMax
                                  ? state.posts.length
                                  : state.posts.length + 1,
                              itemBuilder: (context, index) {
                                return index >= state.posts.length &&
                                        !state.hasReachedMax
                                    ? BottomLoader()
                                    : PostTile(post: state.posts[index]);
                              },
                            ),
                          )
                  ],
                );
              } else {
                return Container();
              }
            }),
      ),
      MediaQuery.of(context).size.width > BREAK_POINT_WIDTH
          ? SideMenu()
          : Container()
    ]);
  }
}
