import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';

import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/components/UserButton.dart';
import 'package:pixel_flutter_web/components/TopicTile.dart';
import 'package:pixel_flutter_web/components/UserDrawer.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';

import 'package:pixel_flutter_web/models/Category.dart';

const BREAK_POINT_WIDTH = 930.0;

class TopicsPage extends StatefulWidget {
  TopicsPage({Key key, this.category}) : super(key: key);

  final Category category;

  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  TopicsBloc _topicsBloc;
  Category category;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    category = widget.category;
    _topicsBloc = TopicsBloc();
    _topicsBloc.dispatch(GetTopics(categoryId: category.id));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicsBloc.dispose();
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
              title: Text(category.name),
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
              child: Layout(
                  topicsBloc: _topicsBloc, controller: _scrollController),
            ),
          );
        });
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicsBloc.dispatch(GetTopics(categoryId: 1));
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

class Layout extends StatelessWidget {
  final TopicsBloc topicsBloc;
  final ScrollController controller;

  Layout({this.topicsBloc, this.controller});

  @override
  Widget build(BuildContext context) {
    return Row(mainAxisAlignment: MainAxisAlignment.center, children: <Widget>[
      Container(
        width: MediaQuery.of(context).size.width > 700
            ? 700
            : MediaQuery.of(context).size.width,
        child: BlocBuilder(
            bloc: topicsBloc,
            builder: (context, state) {
              if (state is TopicsLoaded) {
                return state.topics.isEmpty
                    ? Container()
                    : ListView.builder(
                        controller: controller,
                        itemCount: state.hasReachedMax
                            ? state.topics.length
                            : state.topics.length + 1,
                        itemBuilder: (context, index) {
                          return index >= state.topics.length &&
                                  !state.hasReachedMax
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
