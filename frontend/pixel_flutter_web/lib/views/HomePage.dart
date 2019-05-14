import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicBlocs.dart';
import 'package:pixel_flutter_web/blocs/CategoryBlocs.dart';

import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/components/UserButton.dart';
import 'package:pixel_flutter_web/components/TopicTile.dart';
import 'package:pixel_flutter_web/components/UserDrawer.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';

import 'package:pixel_flutter_web/style/text.dart';
import 'package:pixel_flutter_web/style/colors.dart';

import '../env.dart';

const BREAK_POINT_WIDTH = 930.0;

class HomePage extends StatefulWidget {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  TopicBloc _topicBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    BlocProvider.of<CategoryBloc>(context).dispatch(LoadCategories());
    _topicBloc = TopicBloc();
    _topicBloc.dispatch(GetTopics(categoryId: 1));
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
    return WillPopScope(
      onWillPop: onWillPop,
      child: BlocBuilder(
          bloc: BlocProvider.of<UserBloc>(context),
          builder: (context, userState) {
            return Scaffold(
              endDrawer: userState is UserLoaded ? UserDrawer() : null,
              persistentFooterButtons: <Widget>[],
              appBar: AppBar(
                elevation: 5.0,
                title: Text("pixelshare example"),
                leading: IconButton(
                  onPressed: () => BlocProvider.of<ErrorBloc>(context)
                      .dispatch(GetSuccess(success: "You pressed something")),
                  icon: Icon(Icons.apps),
                ),
                actions: <Widget>[UserButton()],
              ),
              body: BlocListener(
                bloc: BlocProvider.of<ErrorBloc>(context),
                listener: (BuildContext context, ErrorState state) async {
                  snackbarController(context, state);
                },
                child: Layout(
                    topicBloc: _topicBloc, controller: _scrollController),
              ),
            );
          }),
    );
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(GetTopics(categoryId: 1));
    }
  }

  Future<bool> onWillPop() {
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

class Layout extends StatelessWidget {
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
                return ListView.builder(
                  controller: controller,
                  itemCount: state.hasReachedMax
                      ? state.topics.length
                      : state.topics.length + 1,
                  itemBuilder: (context, index) {
                    return index >= state.topics.length && !state.hasReachedMax
                        ? BottomLoader()
                        : TopicTile(topic: state.topics[index]);
                  },
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

